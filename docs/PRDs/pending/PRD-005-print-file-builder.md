# PRD-005 — Print-file builder: complete SC-F2000 jobs from dots and settings

**Status**: Planning
**Owner**: Numbat Web + Claude
**Created**: 2026-11-25 · **Updated**: 2026-11-25
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §4 phase 4, §8 · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)
**Depends on**: PRD-001 (`BandSource`, `encode_bands`), PRD-002 (setup block). PRD-004 supplies real dots, but this PRD is tested with decoded reference dots, so it can start before PRD-004.

> Forward-looking INTENT. Once building starts, don't quietly rewrite it; record reality in
> the decision log.

### Core principles

1. **Context is King** · 2. **Validation Loops** · 3. **Information Dense** ·
4. **Progressive Success** · 5. **Global Rules** (`CLAUDE.md`).

---

## Goal

`f2000::job` writes a **complete** SC-F2000 print file from three things:
1. a job spec: preset, platen, placement, quality, white layers, job name;
2. one `BandSource` per page;
3. the owner's setup block (PRD-002).

It streams straight to disk, and its output matches Garment Creator's byte for byte apart
from documented fields (names and thumbnails).

## Why

- **Business value**: with PRD-003 and PRD-004 this makes printing possible without Garment
  Creator. Today we can only rebuild the raster inside an existing Epson file.
- **Integration**:
  - Reuses the proven writer (`f2000::prn::write_item` and `raster::encode_bands`).
  - The app's Save print file (PRD-006) and the sender (PRD-007) call it.
- **Problems solved**: headers, settings blocks, page setup and thumbnails are currently
  copied from Epson files (`f2000::raster::rebuild`). A new job has nothing to copy from.

## What

### The file it writes (see `docs/print-file-format.md`)

1. **Lead-in:** 16,384 × `0x11`.
2. **EJL preamble.**
3. **`ESC @`, then the job block:**
   - `JS` and `JH` (job name);
   - `JX`;
   - `JU` (application name);
   - three `HD` lines (application, operating system, application id);
   - `MI` and `DN`.
4. **Thumbnail block:**
   - entry 1: the 140 × 168 platen picture;
   - entry 2: a u32 length, the design thumbnail, then the job's settings as URL-encoded
     JSON.
5. **Per page** (white page(s) then colour, or colour only for light presets):
   - `ESC @`;
   - the layer marker `EX … 38 0P` (jobs with white only);
   - `DR`, `DR`, `IK 00 61` (white) or `IK 00 01` (colour);
   - the setup block;
   - `ESC @`;
   - the ESC/P2 page setup:
     - `( G`, `ESC U`, `( K`;
     - `( i` with the quality byte: `F0`/`F1`/`F2`, or `F3` for 720-dpi white;
     - `( e` dot size: `21` white, `11` colour;
     - `( E`;
     - `( U` and `( D` (resolution);
     - `( C`, `( c`, `( S` (page length and width);
   - the raster from `encode_bands`;
   - `FF`.
   - A second white layer repeats the white page with only its layer marker, no ink setup.
6. **Trailer:** `ESC @`, `EX … 38 00`, `LD` and `JE`, `ESC @`.

### Decisions to make while building (record each in the decision log)

- **Application name in `JU`/`HD`:** our own ("DTG Creator Max") is the honest default.
  Test it on the printer (PRD-007). Fall back to a compatible value only if the printer
  rejects it, and record why.
- **Settings JSON in thumbnail entry 2:** write the keys Garment Creator needs to reopen the
  file, with neutral paths, never the user's absolute paths.
- **Page length rounding:** the length in 1/360 in = design top + design height. Confirm
  floor or round from the reference set's fractional `imagePosY`.
- **Constants:** `JX`, `MI`, `DN` and `DR` have never changed across 94 reference files.
  Write them as named constants, each with a doc comment citing the evidence.

### Engineering requirements

- **Streaming:**
  - Writes through a `BufWriter` to a temporary file in the target folder, then renames it
    (atomic: a failed save never leaves half a file).
  - Memory is independent of job size.
- **No Epson data in `f2000`:** the setup block is a parameter (`&[u8; 4775]`) supplied by
  `owner-data`.
- `f2000` keeps its single dependency (`miniz_oxide`, for the thumbnail PNGs).

### Success criteria

- [ ] **Regeneration test**, the referee:
  - For each of the 94 private reference print files, rebuild it from its decoded dots
    (`Page` as `BandSource`), its recorded settings and the setup block.
  - Result: byte-identical to Garment Creator's, except `JS`, `JH`, `JU`, `HD` and the
    thumbnail block. The test masks exactly those items and nothing else.
- [ ] Covers every layout in the set: dark (white + colour), light (colour only), two white
  layers, 720-dpi white, and ink sets 0/1/4.
- [ ] Covers the large platen too. **Reference gap:** every reference print file so far
  uses the medium platen. Generate large, small and extra-small platen variants with the
  private reference builder **before the iMac is wiped**.
- [ ] Peak memory < 32 MB beyond the band source while writing the matched job.
- [ ] An interrupted save (killed mid-write in a test) leaves no partial `.prn`.
- [ ] `f2000 verify` passes on every file we write.

---

## Context & references

### Must-read before implementing

```yaml
- doc: ../../print-file-format.md
  section: Layout; Raster; How Garment Creator writes the raster
- file: crates/f2000/src/prn.rs
  why: Item enum, write_item, RemoteCmd, thumbnail_entries: the building blocks
- file: crates/f2000/src/raster.rs
  why: encode_raster/encode_bands, PageFormat, CHANNEL_ORDER, rebuild (what this PRD replaces for new jobs)
- file: crates/f2000/src/gcr.rs
  why: settings keys and values (printModeColor/White, imagePosX/Y, resizeWidth/Height, platenType)
- file: crates/f2000/tests/golden.rs
  why: reference-set discovery and skip behaviour to mirror
- private: $DTG_REFERENCE/refset/*/call.json + .prn, $DTG_REFERENCE/prn/*.prn + .gcr
  why: settings and the exact expected output for 94 files
```

### Files to create/modify

```bash
crates/f2000/src/job.rs          — CREATE — JobSpec, write_job(), constants with evidence comments
crates/f2000/src/thumbnail.rs    — CREATE — platen panel + design thumbnail PNGs, settings JSON
crates/f2000/src/lib.rs          — MODIFY — pub mod job; error variants
crates/f2000/src/main.rs         — MODIFY — `f2000 build <job.gcr|spec.json> <out.prn>` (dev/test tool)
crates/f2000/tests/regenerate.rs — CREATE — the regeneration test (skips without reference set)
```

### Known gotchas

```
# The setup block is identical for every page and job, but still passed in, never embedded.
# The second white layer reuses the first white page's DR/IK block. Its page gets only the
#   layer marker.
# 720-dpi white (mode 300210) changes ( i to F3 AND ( U / ( D for that page.
# Page length is the DESIGN's bottom, not the platen's: imagePosY + height.
# A colour-only job has no layer markers at all, and the trailer has no `EX … 38 00` block.
# Thumbnail entry 2 holds absolute paths in Garment Creator's files. Ours must not.
```

---

## Implementation blueprint

### Data models

```rust
pub struct JobSpec {
    pub name: String,
    pub platen_width_360: u32,          // 5040 medium, 5760 large
    pub page_length_360: u32,           // design top + design height
    pub colour_quality: u8,             // 0..2 → F0..F2
    pub white: Option<WhiteSpec>,       // None for light presets
    pub thumbnails: Thumbnails,         // platen panel PNG, design thumbnail PNG, settings JSON
}
pub struct WhiteSpec { pub layers: u8, pub fast_720: bool }
pub fn write_job<W: Write>(out: W, spec: &JobSpec, setup_block: &[u8; 4775],
                           pages: &mut [&mut dyn BandSource]) -> Result<(), JobError>;
pub fn save_job_atomically(path: &Path, …) -> Result<(), JobError>;
```

### Tasks (in order)

```yaml
Task 1: JobSpec + constants; derive each constant from the reference set (a test asserts it is in every file)
Task 2: write_job for the light (colour-only) layout; regeneration test on *__light runs
Task 3: White layouts: dark, 2 layers, 720-dpi white; regeneration test on all dark variants
Task 4: Thumbnails and settings JSON (ours); mask list in the test frozen and documented
Task 5: Atomic save; interrupted-write test
Task 6: `f2000 build` dev command; decision-log entry with the JU/HD decision pending printer test
```

### Integration points

```yaml
CONFIG:
  - none (setup block from owner-data)
DEPLOY:
  - none
```

---

## Validation loop

### Level 1 — syntax & style

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```

### Level 2 — unit tests

```bash
cargo test -p f2000                 # synthetic jobs: structure, page count, markers, trailer
```

### Level 3 — integration

```bash
cargo test -p f2000 --test regenerate -- --nocapture   # with $DTG_REFERENCE: 94/94 identical outside the masked items
./target/release/f2000 build "$DTG_REFERENCE/prn/numbatweb-comau-white-lge.gcr" /tmp/built.prn && ./target/release/f2000 verify /tmp/built.prn
```

### Level 4 — manual verification

- [ ] Garment Creator (on the iMac, while it exists) opens a file we built and shows the
  right placement.
- [ ] Once the printer is available (PRD-007): the printer accepts and prints a file we
  built.

---

## Completion checklist

- [ ] All validation levels green
- [ ] `docs/print-file-format.md` updated with anything new (constants' meanings if found,
  the rounding rule)
- [ ] **Decision-log entry appended**
- [ ] PRD moved to `completed/`; index regenerated; NEXT-SESSION updated

---

## Anti-patterns to avoid

- Don't copy headers out of a reference file at run time. Every byte must come from code,
  documented constants or owner data.
- Don't widen the regeneration test's mask to make it pass. Fix the builder.
- Don't buffer whole pages before writing.

---

## Confidence score: 8/10

Every structure is already decoded and verified, and the regeneration test is exact. The
main unknown is whether the printer cares about the application name. That's a printer test,
with a cheap fallback.
