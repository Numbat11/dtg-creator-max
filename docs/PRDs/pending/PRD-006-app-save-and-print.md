# PRD-006 — App: background work, Save print file, Save job, and the Print button

**Status**: Planning
**Owner**: Numbat Web + Claude
**Created**: 2026-11-25 · **Updated**: 2026-11-25
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §5, §6 (journeys 1–3) · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)
**Depends on**: PRD-002 (owner data), PRD-003 + PRD-004 (dots), PRD-005 (print files). Print also needs PRD-007.
**Fixes**: BUG-001 (opening a large print file freezes the window)

> Forward-looking INTENT. Once building starts, don't quietly rewrite it; record reality in
> the decision log.

### Core principles

1. **Context is King** · 2. **Validation Loops** · 3. **Information Dense** ·
4. **Progressive Success** · 5. **Global Rules** (`CLAUDE.md`, copy rules: en-AU, a
disabled control says why).

---

## Goal

All slow work runs off the UI thread, with progress and cancel. **Save print file** turns
the job on screen into a finished `.prn`. **Save job** writes a job file that opens again
here and in Garment Creator. **Print** sends the job to a connected printer. To make this
testable, the app is split into a UI-free **core** and a thin **egui** layer.

## Why

- **Business value**: this completes journeys 1 and 2 in the kickoff brief (artwork or job
  → print). It's the point where DTG Creator Max replaces Garment Creator day to day.
- **Integration**: the buttons already exist, disabled with tooltips, in
  `crates/dtg-creator-max/src/app.rs` (toolbar and `job_footer`). Opening files is in
  `files.rs`.
- **Problems solved**:
  - BUG-001: decoding blocks the UI.
  - App logic can't be tested without a window.
  - Nothing can be saved yet.

## What

### Behaviour

- **Background worker:**
  - One worker thread with a command queue (`std::sync::mpsc`). Commands: open a file,
    build a print file, save a job, send to the printer.
  - Progress messages wake the UI with `ctx.request_repaint()` only when something
    changed; no timers or polling.
  - Cancel is an `AtomicBool` checked once per band.
  - Progress shows in the printer bar's progress bar and the status line, for example
    "Making print file: 42 %".
- **Save print file** (Ctrl/Cmd+Shift+S):
  1. Choose a path.
  2. The job runs colour engine → halftone → print-file builder, streamed to a temporary
     file and renamed when done.
  3. Then offer **Show the dots**, which opens the result as a print-file preview, reusing
     the existing `.prn` view.
- **Save job** (Ctrl/Cmd+S): writes a Garment Creator-compatible `.gcr`: header, the
  artwork PNG, then the settings JSON with our values and neutral paths.
  - Reopening it here restores the job exactly: round-trip tested.
  - Garment Creator compatibility is checked on the iMac while it exists, then on any
    Mac or PC.
  - **Decision recorded here:** reuse `.gcr` rather than invent a format. There's nothing
    new to maintain, and jobs move both ways between tools.
- **Print** (Ctrl/Cmd+P):
  - Enabled only when PRD-007 reports a connected printer and all owner data is present.
  - Otherwise the tooltip names exactly what's missing: printer, owner data, or a design.
- **Remembered preferences:** last preset, platen, folders and window size, stored in a
  small plain-text `key = value` file in the platform config folder. Std only, no
  serialisation crates.

### Engineering requirements

- **The core/UI split:**
  - `core` holds the job model (from `model.rs`), commands, the worker and file
    operations, with no egui types.
  - `ui` turns core state into widgets.
  - Core is tested with plain `cargo test`; the UI with `--screenshot` checks.
- **Never block the UI thread** on file or CPU work. A debug assertion catches long frames
  (> 100 ms) in development.
- **Atomic writes** for every save.
- **No new dependencies**, unless writing the `.gcr` PNG needs more than `image`, which
  already writes PNG.

### Success criteria

- [ ] Opening a 60 MB print file keeps the window responsive: no frame over 100 ms
  (measured with the debug frame-time check). BUG-001 closed.
- [ ] Save print file on the matched job produces a `.prn` that passes `f2000 verify` and,
  once PRD-007 exists, prints.
- [ ] Cancel stops within one band and leaves no file behind.
- [ ] Save job → reopen gives the identical job: placement, preset, platen and every
  setting. Unit-tested in core.
- [ ] A saved `.gcr` opens in Garment Creator 2.1.7 with the right placement (manual, on the
  iMac or any Mac/PC).
- [ ] Every disabled button's tooltip names the missing piece.
- [ ] Idle CPU use is about 0 % with the window open: no repaint loop.

---

## Context & references

### Must-read before implementing

```yaml
- file: crates/dtg-creator-max/src/app.rs
  why: toolbar(), job_footer(), take_screenshot() (request_repaint pattern), NOT_YET tooltips
- file: crates/dtg-creator-max/src/files.rs
  why: open_* functions move into core and onto the worker
- file: crates/dtg-creator-max/src/model.rs
  why: Job, PrintSettings, Preset.print_modes(): the job model becomes core's
- file: crates/f2000/src/gcr.rs
  why: the .gcr layout to write (header: 1, 0, 2, length after the 4th u32, PNG length; PNG; URL-encoded JSON)
- doc: ../../BUGS_BUNNY.md
  section: BUG-001
- url: https://docs.rs/egui/0.37.0/egui/struct.Context.html#method.request_repaint
  why: wake the UI only on progress
```

### Files to create/modify

```bash
crates/dtg-creator-max/src/core/{mod,job,worker,save,prefs}.rs — CREATE — UI-free logic
crates/dtg-creator-max/src/ui/{app,canvas,panels}.rs           — CREATE — split from app.rs/canvas.rs
crates/dtg-creator-max/src/files.rs                            — MODIFY → core/open.rs
crates/f2000/src/gcr.rs                                        — MODIFY — add write() (inverse of parse)
docs/BUGS_BUNNY.md                                             — MODIFY — close BUG-001
```

### Known gotchas

```
# egui 0.37: App::ui, Panel::…show(ui); no emoji glyphs (text labels only). See CLAUDE.md.
# GPU textures are made on the UI thread. The worker returns pixel buffers, and the UI
#   uploads them (ctx.load_texture) when it receives them.
# rfd dialogs block. Open them from the UI thread (fine), but never run work in the same frame.
# .gcr settings must not contain the user's absolute paths when saved by us.
```

---

## Implementation blueprint

### Data models

```rust
pub enum Command { Open(PathBuf), SavePrintFile(PathBuf), SaveJob(PathBuf), Send { printer: PrinterId } }
pub enum Event { Progress { what: &'static str, fraction: f32 }, Opened(OpenResult), Saved(PathBuf),
                 Failed(String), Cancelled }
pub struct Worker { tx: Sender<Command>, rx: Receiver<Event>, cancel: Arc<AtomicBool> }
```

### Tasks (in order)

```yaml
Task 1: Split core/ui with no behaviour change (screenshot check identical)
Task 2: Worker + progress + cancel; move opening onto it (closes BUG-001)
Task 3: gcr::write + Save job + round-trip tests
Task 4: Save print file wired to engine + builder; Show the dots
Task 5: Preferences file; keyboard shortcuts; tooltips naming missing pieces
Task 6: Print button wired to PRD-007's sender when it lands
```

### Integration points

```yaml
CONFIG:
  - platform config folder (XDG_CONFIG_HOME / ~/Library/Application Support / %APPDATA%)
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
cargo test -p dtg-creator-max        # core: job round-trip, worker cancel, atomic save, prefs
```

### Level 3 — integration

```bash
./target/release/dtg-creator-max "$DTG_REFERENCE/prn/numbatweb-comau-white-lge.gcr" --screenshot /tmp/a.png
# Expected: unchanged layout. Then interactively: Save print file → f2000 verify on the result.
```

### Level 4 — manual verification

- [ ] The window stays smooth while a 60 MB print file opens and while a job is built.
- [ ] Cancel works mid-build. No leftovers.
- [ ] Keyboard: open, save, save print file and print all work from shortcuts; Tab reaches
  every control.
- [ ] en-AU copy; every disabled control explains itself.

---

## Completion checklist

- [ ] All validation levels green
- [ ] BUG-001 marked ✅ FIXED with a pointer here
- [ ] **Decision-log entry appended** (the core/UI split, the `.gcr` decision, the
  frame-time results)
- [ ] PRD moved to `completed/`; index regenerated; NEXT-SESSION updated

---

## Anti-patterns to avoid

- Don't poll for progress with a repaint timer.
- Don't invent a job format while `.gcr` works.
- Don't let egui types leak into core.

---

## Confidence score: 7/10

The worker, saving and the split are routine. The risks are outside this PRD: engine
quality (PRD-003/004), and Garment Creator rejecting our `.gcr` for a missing or odd key.
That's testable early with an empty-design job.
