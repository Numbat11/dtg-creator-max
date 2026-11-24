# PRD-003 — Colour engine: artwork to ink separations and white

**Status**: Planning
**Owner**: Numbat Web + Claude
**Created**: 2026-11-25 · **Updated**: 2026-11-25
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §2 (success metrics), §4 phase 3, §7 · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)
**Depends on**: PRD-001 (streaming, lints), PRD-002 (garment colour profiles)

> Forward-looking INTENT. Once building starts, don't quietly rewrite it; record reality in
> the decision log.

### Core principles

1. **Context is King** · 2. **Validation Loops** · 3. **Information Dense** ·
4. **Progressive Success** · 5. **Global Rules** (`CLAUDE.md`).

---

## Goal

A pure-Rust, I/O-free `dtg-engine` crate whose `colour` module turns RGBA artwork plus
job settings into **ink separations**: cyan, magenta, yellow and black amounts, an
**underbase white** and a **design white**, each 8-bit at the artwork's resolution. Each
stage is checked against what Garment Creator produces for the same input.

## Why

- **Business value**: this is the first half of printing without Garment Creator.
  Prints only look right on a shirt if colour, white and transparency behave like Epson's,
  or better.
- **Integration**: the input comes from `files.rs` (decoded artwork) and `model.rs`
  (`PrintSettings`, `Preset`). The output feeds PRD-004 (halftoning).
- **Problems solved**: no artwork-to-ink path exists today. The app can only show artwork
  and existing print files.

## What

### Pipeline (one stage per function, each testable alone)

1. **Adjust:** brightness, contrast, saturation and red/green/blue levels on the RGB
   values (`PrintSettings`).
2. **Colour-manage:** artwork RGB (its embedded ICC profile, or sRGB) to the printer's CMYK,
   through the owner's garment profile (dark 1-layer, dark 2-layer or light; PRD-002) with
   `moxcms`, at the preset's rendering intent (Garment Creator uses relative colorimetric).
3. **Garment black** (ink set 4 presets): pure black becomes no ink, so the shirt shows
   through. Greys still get black ink. The cut-off is measured from the reference set.
4. **Transparency:** alpha scales colour and white. The response curve is fitted from the
   reference set's red-to-transparent ramp, or read from the owner's transparency tables
   if fitting can't match.
5. **White:**
   - **Design white:** light or white areas of the artwork, on dark garments.
   - **Underbase mask:** wherever colour ink lands.
   - **Underbase amount:** from colour and alpha.
   - **Inset:** the underbase is shrunk by `underbase_inset` pixels (Garment Creator's
     `underShapeWidth`), so white never peeks out at the edges.
6. **Density:** colour, underbase and design-white density steps scale ink. Calibrated from
   the reference set: cyan +2 gives about 1.9 % more dots; underbase +2 about 0.4 %.

### Engineering requirements (sustainability rules from PRD-001)

- **Pure functions, deterministic:** same input, same bytes, on every OS. No floating-point
  sums whose order depends on thread scheduling; use fixed-point or per-row results.
- **Streaming by rows:**
  - The engine produces rows on demand for the halftoner, keeping only a window of rows.
  - The window is the inset radius plus the resampling support.
  - Memory scales with the image's width, not its area. Budget: **< 128 MB** peak for a
    14 × 16 in job from 300 dpi artwork.
- **Parallel without a runtime:** row chunks across `std::thread::scope`.
- **One new dependency:** `moxcms` (pure Rust, BSD-3/Apache-2.0, already in the tree via
  `image`). No C CMS.
- **Discovery before building:** the stages above are hypotheses until Task 1 confirms
  them against Epson's intermediate images.

### Success criteria

- [ ] **Masks and white** match the reference set's intermediates on all 90 runs: mean
  absolute difference ≤ 2/255 per pixel, 99th percentile ≤ 16/255. That covers underbase
  mask, underbase white, design white and colour mask.
- [ ] **Colour-managed RGB** matches the reference "colour-managed" image: mean ΔE2000
  ≤ 2.0 and 95th percentile ≤ 4.0 over opaque pixels.
- [ ] **Garment black:** test image 01 (black square) gives zero ink with ink set 4 and
  full black with ink sets 0 and 1.
- [ ] **Property tests pass:**
  - Fully transparent gives zero ink.
  - White on a light preset gives zero ink.
  - Darker never means less total colour ink.
  - A bigger inset never means more underbase.
- [ ] Deterministic: identical output bytes on Linux, macOS and Windows in CI (hash
  compare).
- [ ] Peak memory < 128 MB and < 10 s for the 14 × 16 in matched job on the 2014 iMac.

---

## Context & references

### Must-read before implementing

```yaml
- url: https://github.com/awxkee/moxcms
  why: building RGB→CMYK transforms from ICC profiles (LUT-based profiles, rendering intents)
- url: https://www.color.org/specification/ICC.1-2022-05.pdf
  why: intents and the A2B/B2A tables the garment profiles use
- file: crates/dtg-creator-max/src/model.rs
  why: PrintSettings, Preset, Inks, garment_black: the settings this engine consumes
- file: crates/dtg-creator-max/src/files.rs
  why: how artwork is decoded today (image crate) and its dpi worked out (png_dpi)
- doc: ../../print-file-format.md
  section: Job files: settings that matter for printing
- private: the reference set ($DTG_REFERENCE/refset/<image>__<variant>/)
  why: per run, the exact settings, the print file and Epson's intermediates
    (colour-managed RGBA, colour mask, design white, underbase mask, underbase white).
    Six test images × 15 variants, including inset 0/2/4, no underbase, no design white,
    density changes, ink sets 0/1/4.
```

### Current codebase structure

```bash
crates/f2000/            # format only; no colour code
crates/dtg-creator-max/  # app; artwork decoded in files.rs
```

### Files to create/modify

```bash
crates/engine/Cargo.toml                — CREATE — package dtg-engine; deps: moxcms
crates/engine/src/lib.rs                — CREATE — public API, errors, docs
crates/engine/src/colour/{adjust,manage,white,alpha,black}.rs — CREATE — one stage each
crates/engine/src/rows.rs               — CREATE — row-window streaming
crates/engine/tests/reference.rs        — CREATE — compare stages with the reference set (skips without it)
crates/engine/tests/properties.rs       — CREATE — property tests on synthetic images
tools/refcompare (bin in crates/engine) — CREATE — per-stage diff report + diff images
```

### Known gotchas

```
# REFERENCE GAP (do before the iMac is wiped): the reference set never varied brightness,
#   contrast, saturation or the RGB levels. Generate those variants with the private
#   reference builder now. Otherwise the adjust stage can only be our own formula, and that
#   difference has to be recorded.
# Epson's intermediates are at the ARTWORK's resolution (e.g. 4592×3200 at 216 dpi), not
#   the printer's. Compare at that resolution; resampling belongs to PRD-004.
# The colour-managed intermediate is RGBA, not CMYK. Find out what it represents (Task 1)
#   before trusting it as a target.
# Garment profiles are Epson files: load them only through owner-data (PRD-002), never
#   from a path in the repo.
# Fixed-point or exact integer maths where results must be identical across CPUs.
#   moxcms uses SIMD paths; check the output is the same with the SIMD features on and off.
```

---

## Implementation blueprint

### Data models

```rust
pub struct ColourSettings { /* from PrintSettings + Preset: adjustments, intent, garment_black,
                               inks, inset_px, densities, white_layers */ }
pub struct SeparationRow<'a> { pub c: &'a [u8], pub m: &'a [u8], pub y: &'a [u8], pub k: &'a [u8],
                               pub white_under: &'a [u8], pub white_design: &'a [u8] }
pub struct ColourPipeline { /* profile transform, settings, row window */ }
impl ColourPipeline {
    pub fn new(art: &RgbaImage, profile: &[u8], s: &ColourSettings) -> Result<Self, ColourError>;
    pub fn rows(&mut self, y: usize, n: usize) -> impl Iterator<Item = SeparationRow<'_>>;
}
```

### Tasks (in order)

```yaml
Task 1: Discovery (no product code yet)
  - Write refcompare to load a reference run's artwork, settings and intermediates.
  - Confirm what each intermediate is: is "colour-managed" sRGB → printer → sRGB? Which
    pixels count as colour, design white, underbase? How does the inset map to pixels?
  - Write the findings into this PRD's Notes (while active), then the decision log.
Task 2: Close the reference gap: adjustment variants (before the wipe; private tooling)
Task 3: Crate skeleton, error type, row streaming (no colour yet), property-test harness
Task 4: Stages one at a time, each with its reference comparison: black → alpha → white masks → inset → density → adjust → manage
Task 5: Determinism check in CI (hash of synthetic outputs on three OSes)
Task 6: Benchmarks: time and peak memory on the matched job; record them in the decision log
```

### Pseudocode (only for non-obvious logic)

```
// Underbase inset: a binary erosion of the underbase mask by r pixels, done as two
//   1-D passes (rows, then a sliding window over columns), so it stays O(width) memory
//   with a (2r+1)-row window.
// GOTCHA: r is in Garment Creator's units. Measure from the shape 0/2/4 variants whether
//   it's artwork pixels or 1/360 in.
```

### Integration points

```yaml
CONFIG:
  - none (profiles come from owner-data at run time)
DEPLOY:
  - none
```

---

## Validation loop

### Level 1 — syntax & style

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo deny check
```

### Level 2 — unit tests

```bash
cargo test -p dtg-engine            # properties + synthetic cases; reference tests skip without $DTG_REFERENCE
```

### Level 3 — integration

```bash
cargo run --release -p dtg-engine --bin refcompare -- "$DTG_REFERENCE/refset"
# Expected: a per-run, per-stage table; every row within the thresholds above; diff images saved for failures
```

### Level 4 — manual verification

- [ ] Diff images for the worst five runs look like noise, not structure. No shifted
  edges, no missing white.
- [ ] Soft-proof of the matched job in the app (later, PRD-006) looks right on a dark
  shirt.

---

## Completion checklist

- [ ] All validation levels green
- [ ] `CLAUDE.md`: engine vocabulary (separation, underbase inset, design white)
- [ ] **Decision-log entry appended**, with the Task 1 findings and the measured thresholds
- [ ] PRD moved to `completed/`; index regenerated; NEXT-SESSION updated

---

## Anti-patterns to avoid

- Don't tune constants by eye. Every constant is fitted from the reference set, and the
  fit is in the decision log.
- Don't hold whole images per stage. Stream rows.
- Don't depend on a C library for colour.

---

## Confidence score: 6/10

**Certain:**
- The reference intermediates give a direct, per-stage target for white and masks.
- `moxcms` handles CMYK LUT profiles.

**Assumed:**
- The colour-managed intermediate is a usable target.
- Transparency can be fitted from one ramp.

**Could force a re-plan:** Epson's white generation using rules we can't infer from 90 runs.
Then the transparency tables (PRD-002) become required.
