# PRD-000 — Project Kickoff Brief: DTG Creator Max

**Status**: Confirmed (Numbat Web's own product; the owner is the client)
**Client**: Numbat Web
**CRM Job**: N/A — internal product, no CRM job · **Source**: direct (the owner's own printer)
**Confidential**: no — public repository, GPL-3.0-or-later
**Created**: 2026-11-25 · **Updated**: 2026-11-25

> This is the **first prompt**. It contains everything needed to continue the build.
> Gaps are written as "N/A — reason" so a reader knows they are deliberate. This repository
> is public, so no personal details, CRM data or commercial figures go in it.

---

## 1. Client details (CRM-mapped)

| Field | Value | CRM field |
|---|---|---|
| Contact name | N/A — own product; no personal details in a public repo | `name` |
| Email | N/A — as above | `email` |
| Business | Numbat Web | `business` |
| Phone | N/A — as above | `phone` |
| Services requested | Software system: a desktop app for a DTG printer | `services` |
| Confidential | No — open source | `confidential` |
| Lead source | Direct: the owner's own need | `source` |
| Brief summary + transcript | Summarised below | `message` |

**Brief summary:**

> Numbat Web owns an Epson SureColor SC-F2000 (White Edition) direct-to-garment printer
> and is moving its machines to Ubuntu. Epson's software for the printer, Garment Creator,
> runs only on Windows and macOS, and the printer is discontinued. Running Garment Creator
> in a Windows virtual machine or under Wine was rejected: VMs look poor and lag, and Wine
> "in practice never works". So: build native software that prints to the F2000 from
> Linux, and share it as open source so other owners can print well too.

## 2. Project overview

- **Elevator pitch**: open-source, native Linux software that takes your artwork, places
  it on the platen and prints it on an Epson SC-F2000.
- **Deliverable type**: software system (desktop app plus a print-file library).
- **The problem it solves**: SC-F2000 owners on Linux have no way to print. The only
  software is Windows/macOS-only, and the printer is out of support.
- **Business goals**:
  - Numbat Web can print from Ubuntu.
  - Other owners can too.
  - Numbat Web can earn from support, ready-made builds and separate commercial licences,
    while the software stays free.
- **Success metrics**:
  - Print files are accepted by the printer.
  - Prints look the same as Garment Creator's on real garments.
  - It runs on Ubuntu with no Wine or VM.
  - Garment Creator's own print files can be reproduced byte for byte (already true for
    the writer).
- **Budget band**: N/A — internal product; commercial plans are kept out of this public repo.
- **Timeline / hard deadlines**: none, except one. The only macOS machine with Garment
  Creator (the reference for correctness) is being wiped for Ubuntu. Reference data had to
  be captured before then; that's done for the two main garment presets.
- **Payment milestones**: N/A — internal.

## 3. Audience & positioning

- **Target users**: owners of the Epson SC-F2000, starting with Numbat Web, on Linux
  desktops. Later, the same people on macOS and Windows.
- **Geography & locale**: en-AU spelling in the app ("colour", "centre"). Inches for
  platen sizes and positions, as DTG printing uses; millimetres can come later.
- **Competitors / adjacent products**:
  - Epson Garment Creator: Windows and macOS only.
  - Commercial DTG RIP software: mostly Windows, and paid.
  - DTG Creator Max differs by being open, native on Linux, and fast on old hardware.
- **Tone of voice**: plain and friendly; say what the software can and can't do yet.

## 4. Scope

**In scope (v1):**
- Open artwork (PNG, JPEG), Garment Creator job files (`.gcr`) and print files (`.prn`).
- Place and size the design on the platen; garment presets; print settings.
- Turn artwork into SC-F2000 dots: colour conversion, white underbase and design white,
  halftoning.
- Write complete print files.
- Send them to the printer over USB from Linux.

**Out of scope (for now):**
- Printers other than the SC-F2000 White Edition (the Colour Edition, F2100, F3000…).
- DTF film printing, ink-cost estimates, colour replacement (a placeholder tab exists).
- Shipping or bundling any Epson software or data. Data the printer needs is read from the
  owner's own Garment Creator files.

**Phases:**
1. Print-file library: read, render and write SC-F2000 print files. **Done.**
2. Desktop app shell: layout, files, placement, presets. **First version done.**
3. Colour and halftone engine: artwork to dots.
4. Complete print files for new jobs, and a USB sender.
5. Packaging for Linux, then macOS and Windows.

## 5. UI — look & feel

- **Build style / starting template**: N/A — native desktop app, not a website. At the
  owner's request the layout follows the familiar DTG layout:
  - a printer bar across the top;
  - the garment and platen with rulers in the middle;
  - presets and settings on the right;
  - job name, copies and Print at the bottom.

  Everything is drawn from scratch: no Epson artwork, icons or wording.
- **Colour palette** (from `crates/dtg-creator-max/src/canvas.rs` and `app.rs`):
  - Chrome: `#08080A` printer bar, egui dark theme.
  - Settings panel: `#EEEEEE`, light theme.
  - Labels: `#286EC8`.
  - Platen edge: `#00E0EC`.
  - Selection accent: `#FFA028`.
  - Canvas: `#303236`.
- **Typography**: egui's default fonts. **No emoji or icon glyphs** in this egui version,
  so labels are text.
- **Layout direction**: app shell (a single window).
- **Key components**: printer bar, toolbar, garment canvas with rulers and platen grid,
  preset list, collapsible settings, zoom bar with status line.
- **Motion**: none.
- **Client copy rules**:
  - en-AU spelling.
  - Name Epson products only to say what DTG Creator Max works with. Never imply Epson
    made, endorses or supports it.
  - Greyed-out features say why in a tooltip.
- **Dark mode**: dark chrome with a light settings panel. No theme switch yet.

## 6. UX — flows & quality bars

- **Primary user journeys:**
  1. Open artwork → choose the garment preset → place and size it → print.
  2. Open an existing Garment Creator job → check it → print.
  3. Open a print file → see exactly what will print → send it.
- **Key interactions**:
  - Dragging the design on the platen; exact position and size in Layout settings.
  - Zooming and panning.
  - Dropping files on the window.
- **Accessibility target**: the main flows usable from the keyboard. AccessKit is enabled,
  so screen readers can read the controls.
- **Performance budget**:
  - Smooth on old GPUs (OpenGL, not wgpu).
  - Opening a 50 MB print file in about a second.
  - Building in release mode for real use.
- **Devices**: desktop and laptop screens; minimum window 1100 × 700.
- **Error/empty/loading states**:
  - A clear status line for every file opened or refused.
  - Features that aren't ready are disabled, with the reason.

## 7. Data handling

- **Core entities**:
  - Job: name, copies, preset, platen, settings, artwork.
  - Preset; Platen; PrintSettings; Artwork.
  - For print files: Page, Plane (2-bit dots per ink channel) and Strip.
- **Storage**: local files only (the user's artwork, jobs and print files).
- **User accounts/auth**: none.
- **PII collected**: none. No telemetry, no network access.
- **Privacy tiers/consent**: N/A — nothing collected.
- **Retention & purge**: N/A.
- **Backups**: N/A — users' own files. The source is on GitHub.
- **Compliance**: GPL-3.0-or-later obligations; trademarks used only nominatively.

## 8. APIs & integrations

- **Internal endpoints**: none.
- **The printer**: ESC/P2 with Epson remote-mode commands, sent over USB (IEEE 1284 class).
  The format is documented in [`print-file-format.md`](print-file-format.md).
- **Third-party services**: none.
- **Existing Numbat shared services**: none.
- **Env vars / secrets**: none in the app. Tests read `DTG_REFERENCE`, the folder holding
  the private reference set.
- **Rate limiting / abuse protection**: N/A.

## 9. Graphics & assets

- **Client-supplied**: none in the repo. Customers' artwork never goes in git.
- **To be produced**: an app icon; desktop launcher files (`.desktop`) for Linux.
- **Image treatment**:
  - On screen, artwork is shown at its real print size from the image's resolution.
  - Textures are capped at 2048 px for display only.
- **Video**: N/A.
- **Licensing**: no Epson assets. The T-shirt is a vector drawing in `canvas.rs`.

## 10. References & inspiration

- **Epson Garment Creator 2.1.7**: the functional reference for layout, settings and output.
  It is not a visual source.
- **Gutenprint**: prior art for Epson ESC/P2 printing on Linux. Its RLE is the same family
  as the F2000's.
- **Anti-references**: VMs and Wine wrappers; anything that looks or behaves like a ported
  Windows app.
- **Existing collateral**: none.

## 11. Tech stack & hosting

- **Framework**: none of the three web options. This is a native desktop app:
  - **Rust** (2024 edition, stable), a Cargo workspace.
  - **egui/eframe 0.37** using the OpenGL (`glow`) renderer.
  - `rfd` for file dialogs (the XDG portal on Linux, so no GTK build dependency).
  - `image` (PNG/JPEG) and `miniz_oxide` (PNG output).
  - **Reason not 1:** it needs printer I/O, heavy image processing, a single native binary
    and good speed on old hardware. The owner wants a native app, not a web or VM wrapper.
- **Starting point**: greenfield.
- **Deploy target**: GitHub releases (not set up yet). No server.
- **Domains**: none yet.
- **Repo**: https://github.com/Numbat11/dtg-creator-max, public. No CI yet.

## 12. Content

- **Sitemap / page list**: N/A — one app window, plus the docs in `docs/`.
- **Copy source**: written in the repo, following the copy rules in §5.
- **SEO**: GitHub topics only (dtg, direct-to-garment, linux, rust, printer-driver, epson).
- **Legal pages**: `LICENSE` (GPL-3.0), `CONTRIBUTING.md` (contributor terms), and the
  trademarks note in `README.md`.

## 13. Legal & commercial

- **Licence**: GPL-3.0-or-later, copyright Numbat Web.
  - Contributors agree that Numbat Web may also license their contributions under other
    terms (`CONTRIBUTING.md`). That keeps paid commercial licences possible.
- **Interoperability**: format knowledge comes from print files that Garment Creator made
  on our own machine.
  - No Epson software or data is committed.
  - The fixed 4,775-byte block every print file needs is Epson data. It will be read from
    the owner's own Garment Creator files at run time.
- **IP ownership on handover**: N/A — own product.
- **Ongoing maintenance**: by Numbat Web; community contributions welcome.

## 14. First-session plan

- [x] Kickoff brief written (this file).
- [x] Repo bootstrapped from the hybrid-build seed; `CLAUDE.md` filled.
- [x] Work so far recorded in the decision log (2026-11-24 to 2026-11-25).
- [ ] Initial PRD set agreed, then written into `PRDs/pending/`. **Proposed:**
  - PRD-001 — Colour engine: RGB artwork to the F2000's CMYK and white separations,
    checked against Epson's intermediate images in the private reference set.
  - PRD-002 — Halftoning: separations to 2-bit dot planes with matching ink density per
    tone.
  - PRD-003 — New print files: build complete jobs (header, settings, thumbnail, pages)
    from dots and settings, reading the Epson data block from the owner's files.
  - PRD-004 — App: Save print file, Save job and the Print button wired to PRDs 001–003.
  - PRD-005 — USB sender on Linux, with printer status. Blocked until the printer is
    available.
  - PRD-006 — Packaging and CI: Linux first (AppImage or .deb), then macOS and Windows
    builds.
- [x] Validation gates: `cargo build --release`, `cargo test` (unit tests plus golden tests
  against the private reference set), and a headless screenshot of the app
  (`dtg-creator-max <file> --screenshot out.png`).
- [x] First decision-log entries written.

## 15. Open questions & confidence

**Open questions:**
- The exact settings of the two unconfirmed garment presets ("Dark T-shirt, white ink only"
  and "Light T-shirt, fast colour"). Each needs one print file captured from Garment
  Creator before the iMac is wiped.
- How closely halftoning has to match Epson's. Same look and ink use is the goal;
  byte-identical dots probably isn't achievable without Epson's tables.
- How the printer reports status over USB, and whether it needs anything beyond the print
  file.

**Confidence score**: 6/10.
- **Certain:** the print-file format, the writer, and the app shell.
- **Assumed:** a native colour and halftone engine can match Epson's look closely enough.
- **Could sink the estimate:** halftone quality, and USB behaviour untested without the
  printer.
