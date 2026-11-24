# DTG Creator Max — root orientation

> **Starting fresh? Read [`docs/NEXT-SESSION.md`](docs/NEXT-SESSION.md) first.** It says what
> is published, what is committed but not released, what still needs a human, and the rules
> that bite if forgotten. Kept current at the end of each session.

## What this is

Numbat Web's own open-source (GPL-3.0-or-later) desktop software for printing on the Epson
SureColor SC-F2000 direct-to-garment printer from Linux, with no Wine and no VM. It is a Rust
workspace with two crates: a print-file library and a desktop app. Full scope and reasoning
are in the kickoff brief.

- **Client**: Numbat Web (own product) · CRM job: none · Confidential: no. **This repo is
  public**, so nothing private goes in it.
- **Kickoff brief (PRD-000)**: [`docs/KICKOFF-BRIEF.md`](docs/KICKOFF-BRIEF.md): scope,
  UI/UX, data, stack, legal.
- **PRD workflow**: [`docs/PRDs/README.md`](docs/PRDs/README.md): plan → build → validate →
  record.
- **Decision log**: [`docs/PRDs/00_decision_log.md`](docs/PRDs/00_decision_log.md): what
  actually shipped; append-only.
- **Bug tracker**: [`docs/BUGS_BUNNY.md`](docs/BUGS_BUNNY.md): log out-of-context bugs the
  moment you spot them.
- **Print-file format**: [`docs/print-file-format.md`](docs/print-file-format.md).
- **Docs index**: [`docs/INDEX.md`](docs/INDEX.md): generated; run `npm run generate:docs-index`.

## Tech stack & running it

- **Stack**: Rust (2024 edition, stable), Cargo workspace.
  - `crates/f2000`: print-file library and `f2000` CLI. Only dependency: `miniz_oxide`.
  - `crates/dtg-creator-max`: the app. egui/eframe 0.37 with the OpenGL (`glow`) renderer,
    `rfd` file dialogs (XDG portal on Linux), `image` (PNG/JPEG).
  - **Not the house web stack** because it's a native desktop app with printer I/O and
    heavy image work.
- **Run**: `cargo build --release`, then `./target/release/dtg-creator-max [file]`.
- **Test**: `cargo test`. The golden tests need the private reference set (below) and skip
  without it.
- **Screenshot check**: `./target/release/dtg-creator-max <file> --screenshot out.png`
  saves the window as a PNG and quits.
- **Docs index**: `npm install` once, then `npm run generate:docs-index`. Enable the
  pre-commit check once per clone with `git config core.hooksPath .githooks`.
- **Deploy**: none yet; GitHub releases are planned (PRD-006 proposed).
- **Env vars**: `DTG_REFERENCE` (tests only): the private reference folder. It defaults to
  `./reference`, a git-ignored symlink to the private reference set.
- **Rust toolchain on the iMac**: rustup without PATH changes, so use `~/.cargo/bin/cargo`.

## Vocabulary & client copy rules

- **Print file** (`.prn`): what the printer receives. **Job file** (`.gcr`): Garment
  Creator's saved job (artwork plus settings).
- **Page**: one pass over the garment in a print file. **White page**: the underbase,
  1440 × 1440 dpi. **Colour page**: 1440 × 720 dpi.
- **Plane**: one ink channel's 2-bit dots on a page. **Strip**: one `ESC i` raster command.
  **Band**: 16 lines.
- **Underbase**: white under the colour. **Design white**: white that is part of the
  artwork. **Garment black**: pure black left unprinted so the shirt shows through (ink set
  4 does this).
- **Platen**: the board the garment is loaded on. Positions are in inches from its top-left.
- **Reference set**: print files and intermediates made with Epson's software on our own
  machine. **Private, never committed.**
- **Copy rules**:
  - en-AU spelling ("colour", "centre").
  - Name Epson products only to say what we work with. Never imply endorsement.
  - A disabled control says why in its tooltip.

## Rules that bite

- PRDs are intent; the decision log is reality. Never quietly rewrite a PRD's history.
- **Never commit Epson files**: installers, libraries, profiles, halftone tables, print or
  job files made by Epson software, or customers' artwork. `.gitignore` blocks `.prn`,
  `.gcr`, `.icc` and `.icm`.
- **The 4,775-byte `EX` block** in every print file is Epson data. Read it from the owner's
  files at run time; never ship it.
- **egui 0.37 API**:
  - Panels are `egui::Panel::top/bottom/left/right(id).show(ui, …)`, attached to the
    parent `Ui`.
  - The app implements `eframe::App::ui(&mut self, ui, frame)`, not `update`.
  - Screenshots go through `ctx.request_screenshot(callback)`.
  - The default fonts have **no emoji or icon glyphs**: they render as boxes.
- **Use OpenGL, not wgpu.** wgpu fails with "Device Lost" on the OpenCore-patched iMac;
  `glow` works on old GPUs.
- **Writer rule**: a strip is padded to an even width **only** when compressed. Raw
  (mode 4) strips keep their exact width. Getting this wrong only shows on noisy halftones.
- `docs/INDEX.md` is generated. Edit `docs/index.config.yaml` instead, then regenerate.

## Session-end checklist

1. Update the active PRD's status header.
2. If something shipped, append a decision-log entry.
3. Bugs spotted but not fixed go in `docs/BUGS_BUNNY.md` as `BUG-NNN` entries.
4. Update `docs/NEXT-SESSION.md`: what's published, released, waiting, and needing a human.
5. If docs were added or moved, run `npm run generate:docs-index` (pre-commit verifies anyway).
6. Distil any durable conclusion into this file (Vocabulary / Rules that bite).
