# Decision Log — DTG Creator Max

Append-only record of what was **actually built** and **why**, for contextual referral.
Newest entry on top. **Never edit or delete prior entries**; supersede them with a new
one. Each entry is a lightweight ADR. Keep it PII-free: this repository is public.

PRDs (forward-looking intent) live in `pending/`, `active/` and `completed/` alongside this
file. When a build lands, record reality here; don't rewrite the PRD's history.

The entries dated 2026-11-24 and 2026-11-25 were written on 2026-11-25, when the repository
adopted the Numbat hybrid build docs. That work had no PRDs, so they record it after the
fact from the commits and working notes.

---

<!-- ENTRY TEMPLATE — copy below this comment, newest on top.

## YYYY-MM-DD — {{One-line: what shipped}} (PRD-NNN)

**What shipped.** {{The observable result — feature, fix, change. Present tense, concrete.}}

**Implementation.** {{The approach that was actually taken, and the 1–3 decisions that
mattered. For each: what was chosen, what was rejected, and WHY — the why is the value.}}

**Differs from the PRD?** {{None | what changed vs the plan and what forced it.}}

**Bugs caught / traps worth remembering.** {{Anything a future session would step on.}}

**Verified.** {{How you know it works — commands run, values observed, devices checked.}}

**Open.** {{Anything left deliberately unresolved, with the tuning lever named.}}

-->

## 2026-11-25 — Initial PRD set written; sustainability rules adopted (planning, no code)

**What shipped.** PRD-001 to PRD-008 in `PRDs/pending/`, listed with a build order in
the kickoff brief §14.

**Implementation.**
- **The six proposed PRDs became eight.** Two were added in front:
  - engineering foundations (PRD-001: CI on three OSes, lints, licence policy, a pinned
    toolchain, a memory budget);
  - owner data import (PRD-002: the only path by which Epson files reach the app).
  The proposals had never been written down, so renumbering broke no PRD-number rule.
- **The sustainability rules** live in PRD-001 and apply to all PRDs:
  1. long-lived code (small crates, pure functions, tests as the specification, no
     `unsafe`);
  2. a small pure-Rust dependency tree with GPL-compatible licences;
  3. hardware efficiency (10-year-old machines, band streaming);
  4. energy efficiency (no polling or telemetry);
  5. reproducible, auditable releases;
  6. accessibility.
- **Technical choices made at planning time** (each PRD records its reasons):
  - `moxcms` for colour, because it's pure Rust and already in the tree;
  - multi-level blue-noise dithering over error diffusion, because it parallelises,
    streams and is deterministic;
  - the kernel's `usblp` device on Linux over libusb or CUPS, because it needs no
    dependencies;
  - `.gcr` as the saved-job format over a new format;
  - `cargo-dist` with `cargo-deb` for releases.
- **Building PRD-005 before PRD-003/004** is possible because it's tested against decoded
  reference dots.

**Bugs caught / traps worth remembering.** Writing the PRDs exposed three gaps in the
private reference set, all needing macOS before the iMac is wiped: the two unconfirmed
presets, colour-adjustment variants, and platens other than medium. They're listed in
NEXT-SESSION.

**Open.** Everything; this is planning.

## 2026-11-25 — Project docs in the Numbat hybrid build structure (no PRD)

**What shipped.**
- The repo now has `CLAUDE.md`, `docs/KICKOFF-BRIEF.md` (PRD-000), this decision log,
  `docs/NEXT-SESSION.md`, `docs/BUGS_BUNNY.md`, the PRD templates and folders, and a
  generated `docs/INDEX.md`.
- The index scripts come from the house seed and run with Node (`npm run
  generate:docs-index`). A pre-commit hook in `.githooks/` blocks a stale index.

**Implementation.**
- The seed is built for client websites. Client, CRM and web-only sections are filled as
  "N/A — reason" rather than deleted, so the structure stays recognisable.
- Because the repo is public, the kickoff brief holds no personal details, CRM data or
  commercial figures. Epson-internal research notes stay in the maintainer's private
  folder; only the print-file format is public.
- Node tooling was added only for the docs index (`package.json` with dev dependencies
  only). Rewriting the index generator in Rust was rejected as "a new pattern when an
  existing one works". Contributors who never touch docs don't need Node: the hook skips
  with a warning if `node_modules` is missing.
- The next PRDs are proposed in the kickoff brief §14, not written. The workflow writes
  PRDs after agreement.

**Differs from the PRD?** No PRD.

**Verified.** `npm run generate:docs-index` then `npm run verify:docs-index` passes;
`cargo test` passes.

**Open.** The PRD set in the kickoff brief §14 needs agreement before PRD-001 is written.

## 2026-11-25 — Desktop app, first version (no PRD; commit `d9e4813`)

**What shipped.**
- `crates/dtg-creator-max`: a native window with the familiar DTG layout:
  - a printer bar ("Off-line: no printer connected") with Print settings, Layout settings
    and Colour replacement tabs;
  - a toolbar;
  - a canvas with rulers in inches, a vector T-shirt coloured to the preset, and the
    platen with its 1-inch grid;
  - a settings panel with five garment presets, quality, direction, drying time, white
    layers, underbase inset, ink density and colour adjustments;
  - job name, copies and Print at the bottom.
- **Opening files:** PNG and JPEG at their real print size, from the PNG's `pHYs`
  resolution, or 300 dpi if none is stored. Garment Creator `.gcr` jobs with their
  placement, preset, platen and settings. `.prn` print files, shown as the dots they will
  print.
- **Editing:** drag to move, the Layout tab for exact position and size, pinch or
  Ctrl+scroll to zoom, drag-and-drop to open.
- **Disabled, with the reason in a tooltip:** Print, Save print file and Save job.
- `f2000::render::preview` now estimates fabric coverage per dot size (60/85/100 %), with
  a spread curve, so light prints are readable. Ink totals are unchanged.

**Implementation.**
- **egui/eframe** was chosen over GTK4: pure Rust, no system UI libraries to install,
  the same code on Linux, macOS and Windows, and GPU-drawn.
- **The OpenGL renderer** (`glow`) was chosen over the default `wgpu`. `wgpu` failed on
  the OpenCore-patched iMac with "Device Lost", and OpenGL is the safer bet on older Linux
  machines too.
- **`rfd`** uses its default XDG-portal backend on Linux, so building needs no GTK
  headers.
- **Layout:** the owner asked for Garment Creator's layout. It is reproduced functionally,
  with our own drawing, wording and colours; no Epson images, icons or text.
- **Presets:** garment ids -20 (black, use garment black, ink set 4), -40 (light standard,
  ink set 0) and -60 (dark standard, ink set 1) are confirmed from Epson-made files. "Dark
  T-shirt (white ink only)" and "Light T-shirt (fast colour)" are placeholders, marked
  unconfirmed in a tooltip.

**Bugs caught / traps worth remembering.**
- **egui 0.37 changes:**
  - `egui::Panel::top/right(id).show(ui, …)` replaces `TopBottomPanel` and `SidePanel`.
  - `eframe::App::ui` replaces `update`.
  - Screenshots use `ctx.request_screenshot(callback)`.
- **No emoji in egui 0.37's default fonts:** toolbar icons rendered as boxes, so buttons
  use text.
- **The first preview was far too faint** because it measured ink against "every pixel a
  large dot".

**Verified.**
- Headless screenshots (`--screenshot`) of a `.gcr` job, a black-garment `.prn` and a
  light-garment `.prn` were checked by eye.
- `cargo test`: 20 tests pass.
- Built and run on macOS 15 (Intel, OpenCore); **not yet run on Linux.**

**Open.**
- Linux build and run.
- A `.prn` opens on the UI thread, a short freeze (BUG-001).
- The two unconfirmed presets.
- Colour replacement is a placeholder.

## 2026-11-25 — Writer rule fixed against a larger reference set (no PRD; commit `96a147b`)

**What shipped.**
- `raster::encode_raster` pads a strip to an even width **only when it compresses it**.
  A strip whose compressed form would be larger is sent raw (mode 4) at its exact width.
- With this, 88 print files made by Garment Creator's engine rebuild byte for byte from
  their dots (21,620 strips). They include 720-dpi white, two white layers, colour-only
  jobs, ink sets 0, 1 and 4, and different densities.
- The golden tests now cover `reference/refset/*/*.prn`.
- `docs/print-file-format.md` now explains:
  - the white/colour layer markers;
  - `IK 00 61` (white page) and `IK 00 01` (colour page);
  - print quality as the microweave byte (`F0`/`F1`/`F2`, with `F3` for the 720-dpi
    white);
  - the job settings embedded in the second thumbnail entry.

**Implementation.**
- The reference set: the six test images × 15 settings variants, generated with Garment
  Creator on the maintainer's Mac and kept private. It varies one setting at a time, so
  each difference in the print file can be pinned to a cause.
- The old rule ("pad to even, except a 1-byte strip") matched the first three jobs only by
  luck. It broke on noisy white halftones under black, where Epson sent 709 raw bytes and
  we sent 710.

**Bugs caught / traps worth remembering.**
- The 4,775-byte `EX` block is identical in every job and is Epson data. It must be read
  from the owner's files, never shipped.
- **"Use garment black" is the ink set, not the garment name.** Ink set 4 leaves pure
  black unprinted.

**Verified.** `f2000 verify` on all 88 files: parse and write, re-compression, and
rebuild from dots are all byte-identical. `cargo test` passes.

**Open.** What `JX`, `MI`, `DN` and `DR` mean: unchanged in every job so far.

## 2026-11-25 — Published as open source, GPL-3.0-or-later (no PRD; commit `f992272`)

**What shipped.**
- The public repository https://github.com/Numbat11/dtg-creator-max with the print-file
  library, a format doc, `LICENSE` (GPL-3.0 text, checksum-verified against the FSF copy),
  `CONTRIBUTING.md` and SPDX headers.
- Commits use GitHub's no-reply email address.

**Implementation.**
- **The name** "DTG Creator Max" is the owner's choice.
- **GPL-3.0-or-later over Apache-2.0.** The owner wants everyone to be able to print well
  but needs the work to pay for itself. The GPL keeps the software free to use, including
  for commercial printing, and stops closed-source forks. Numbat Web, as copyright holder,
  can still sell separate licences.
- **Contributor terms:** `CONTRIBUTING.md` asks contributors to agree that Numbat Web may
  also license their contributions under other terms. Without that, dual licensing ends
  with the first outside contribution.
- **Kept out of the repo:** no Epson files, print files or artwork. `.gitignore` blocks
  `.prn`, `.gcr`, `.icc` and `.icm`. The golden tests read a private reference folder and
  skip without it.

**Verified.**
- gitleaks found nothing in the tree or history.
- GitHub detects the licence as GPL v3.0.

## 2026-11-24 — Print-file library: format decoded, jobs rebuilt byte for byte (no PRD)

**What shipped.** The `f2000` crate and CLI:
- lossless parse and write of the SC-F2000 command stream;
- Epson TIFF-mode run-length coding (`0x80` repeats 129 times);
- framed raster modes 4 and 5;
- decoding to full-page 2-bit dot planes per ink channel (white 63, black 0, magenta 1,
  cyan 2, yellow 4);
- previews and exact ink totals;
- `.gcr` job files (artwork plus 74–76 settings);
- `info`, `render`, `verify`, `rebuild` and `gcr` commands.

**Implementation.**
- **Interoperability first.** The format was decoded from print files that Garment
  Creator 2.1.7 saved offline on our own Mac, not from Epson's code.
- **Each page has its own resolution:** white 1440 × 1440, colour 1440 × 720. Reading one
  fixed resolution squashed the colour.
- **The writer follows Garment Creator's exact layout:**
  - 16-line bands on a fixed grid, empty bands skipped;
  - C, M, Y, K order within a band;
  - `ESC ( v` before a band's first strip only;
  - left edge snapped down to 16 pixels;
  - per-line RLE with literals of at most 127 and repeats of at most 128, for runs of 3 or
    more.

  The goal is byte-identical output, which proves the writer before any halftoning exists.
- **Dependencies:** one, for PNG output (`miniz_oxide`).

**Bugs caught / traps worth remembering.** A Python prototype over-counted ink by summing
4-byte groups across line ends. The Rust totals were checked against an exact count.

**Verified.** All three saved jobs (20–60 MB) rebuild byte for byte; Epson's nozzle-check
file round-trips; the render reproduces the job's artwork.

## 2026-11-24 — Direction: a native Rust app, no Wine or VM (no PRD)

**What was decided.**
- SC-F2000 printing on Linux will be **native software**, written in Rust.
- **Rejected:**
  - Running Garment Creator in a Windows VM: the owner finds VMs "visually terrible" and
    laggy.
  - Wine: "in practice it never works".
  - A Wine-hosted copy of Epson's halftone libraries, considered on 2026-11-24 and
    dropped on 2026-11-25 for the same reason.
- No existing open-source Linux driver or RIP supports the SC-F2000.

**Why it matters.**
- Everything Epson does between artwork and dots (colour conversion, white underbase,
  halftoning) must be rebuilt natively.
- Epson's software is useful only as a **reference** for checking our output. It runs on
  macOS, so reference data had to be captured before the maintainer's Mac moved to
  Ubuntu.
