# NEXT-SESSION — DTG Creator Max

> The session handoff. Updated at the **end of every session**. A fresh session (human or
> AI) reads this first and knows exactly where things stand. Keep it short and
> current; history belongs in the decision log, not here.

**Last updated**: 2026-11-25 (PRD-001 to PRD-008 written; docs in the hybrid structure; desktop app v1 shipped)

## Published

- Source is public at https://github.com/Numbat11/dtg-creator-max (`main`).
- **No release builds yet.** People build from source (README).

## Committed but NOT released

- Everything: the print-file library, the `f2000` CLI and the desktop app first version.
  Packaging and releases are proposed as PRD-006.

## Needs a human

- **Run the app on Ubuntu** (the MacBook): it has only been built and run on macOS so far.
  - Follow the README: install `build-essential`, then rustup.
  - Then `cargo build --release` and `./target/release/dtg-creator-max`.
  - Log anything odd in BUGS_BUNNY.
- **Before the iMac is wiped**, using the maintainer's private reference tools (see the
  private notes, not this repo). After the wipe there's no macOS to capture with.
  1. The two unconfirmed garment presets, "Dark Color T-Shirt (White)" and "Light Color
     T-Shirt (High Speed Color mode)": save one print file each from Garment Creator.
  2. Colour-adjustment variants (brightness, contrast, saturation, red/green/blue levels).
     Needed by PRD-003.
  3. Platen variants (large, small, extra small). Every reference file so far is the medium
     platen. Needed by PRD-005.
  4. Import the Garment Creator folder through PRD-002 once it exists, or at least keep a
     copy of the folder off the iMac. It's already in the private `epson-f2000` backup on
     the TRANSFER stick.
- **The printer is not with the owner.** Anything involving USB waits for it.

## In flight

- **Active PRD(s)**: none. PRD-001 to PRD-008 are written, in `PRDs/pending/`.
- **Next up**: [PRD-001](PRDs/pending/PRD-001-engineering-foundations.md), the engineering
  foundations. Move it to `active/` when work starts. Build order is in the kickoff brief
  §14.

## Waiting on the client

- N/A — own product.

## Rules that bite (top 3 right now)

1. **Never commit Epson files, print/job files or anyone's artwork.** The private reference
   set is reached through the git-ignored `reference` symlink or `DTG_REFERENCE`.
2. **egui 0.37 + OpenGL:**
   - `egui::Panel::…show(ui)` and `App::ui`;
   - no emoji glyphs;
   - keep the `glow` renderer, because `wgpu` fails on older GPUs.
3. **`docs/INDEX.md` is generated.** Edit `docs/index.config.yaml`, then run `npm run
   generate:docs-index`; the pre-commit hook checks it.
