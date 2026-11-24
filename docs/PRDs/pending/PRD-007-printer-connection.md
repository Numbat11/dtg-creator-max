# PRD-007 — Printer connection: find the SC-F2000 and send jobs (Linux first)

**Status**: Planning. Hardware steps **Blocked** until the printer is back with the owner.
Everything else can be built and tested without it.
**Owner**: Numbat Web + Claude
**Created**: 2026-11-25 · **Updated**: 2026-11-25
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §4 phase 4, §8, §15 · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)
**Depends on**: PRD-005 (print files to send). PRD-002 for the optional nozzle check.

> Forward-looking INTENT. Once building starts, don't quietly rewrite it; record reality in
> the decision log.

### Core principles

1. **Context is King** · 2. **Validation Loops** · 3. **Information Dense** ·
4. **Progressive Success** · 5. **Global Rules** (`CLAUDE.md`).

---

## Goal

DTG Creator Max **finds** an SC-F2000 connected by USB on Linux, shows it in the printer bar
("Ready" instead of "Off-line"), and **sends** print files to it safely, with progress and
cancel. It also sends over the network where the printer supports that. macOS and Windows
sending follow in a later PRD.

## Why

- **Business value**: this is the last step to printing from Ubuntu, the reason the
  project exists.
- **Integration**:
  - The printer bar and Print button exist in `app.rs`.
  - PRD-006 wires Print to this crate.
  - PRD-005 produces the bytes.
- **Problems solved**: Epson's sender is part of Garment Creator (Windows/macOS only).
  Linux has no way to deliver a job.

## What

### Approach (simplest durable path first)

- **Linux USB: write to the kernel's printer device.**
  - The SC-F2000 is a standard USB printer, so Linux's `usblp` driver exposes it as
    `/dev/usb/lpN`. Writing a print file there prints it: the same as `cat job.prn >
    /dev/usb/lp0`.
  - Needs no USB library and no root. The user must be in the `lp` group; the app explains
    this if access is denied.
  - **Discovery:** read `/sys/class/usbmisc/lp*/device/ieee1284_id` (the printer's IEEE
    1284 device ID). Accept only devices whose `MFG` is EPSON and whose `MDL` names the
    SC-F2000; confirm the exact string on the real printer.
- **Network** (if the printer is networked): raw TCP to port 9100 on an address the user
  enters. Discovery (mDNS) comes later. Confirm the F2000's network support on the printer.
- **Later, in its own PRD:**
  - macOS through `nusb` (pure Rust, Apache/MIT).
  - Windows through the system spooler in RAW mode.
  - Neither needs libusb.
- **Status:**
  - Phase 1 is presence only: found, ready to send, sending, done or error.
  - Ink levels and error codes need Epson's status replies, researched with the printer
    in a later phase.
- **Safety:**
  - Send only to a device whose 1284 ID matches.
  - Confirm before sending: job name, copies, print size.
  - Stream in 64 KB chunks with progress; cancel stops between chunks.
  - Never send a file that fails `f2000::verify`'s structure checks.
- **Copies:** sent as repeated jobs. Whether the format has a copies field is unknown;
  check on the printer.
- **Nozzle check:** a "Print nozzle check" item sends the owner's own nozzle-check file,
  imported through PRD-002 (it's Epson's file, so never shipped). A useful first hardware
  test.
- **Dry run:** "Send to file…" writes exactly the bytes that would go to the printer. Used
  for tests and support.

### Engineering requirements

- New crate `printer-io`. **Zero dependencies on Linux.** `nusb` only later, behind a
  macOS-only `cfg`.
- **Sysfs paths are injectable,** so discovery is tested with a fake tree in a temp folder.
- **Sending streams from disk;** memory doesn't depend on file size.
- **No background polling** for presence. Rescan when the window gains focus, or when the
  user clicks the printer menu. Hot-plug events (udev) are a possible later step.

### Success criteria

- [ ] Discovery finds an SC-F2000 in a fake sysfs tree and ignores other printers (unit
  tests).
- [ ] Sending to a temp file or FIFO delivers the exact bytes, with progress and a working
  cancel (unit tests).
- [ ] Permission denied produces the message "Add yourself to the lp group" with the
  command (unit-tested message).
- [ ] **Hardware (blocked until the printer returns):**
  - The owner's nozzle-check file prints.
  - A job built by PRD-005 prints and matches Garment Creator's print of the same job by
    eye.
  - The printer bar shows the printer as found.
- [ ] No CPU use while idle (no polling).

---

## Context & references

### Must-read before implementing

```yaml
- url: https://www.kernel.org/doc/html/latest/admin-guide/devices.html
  why: usblp character devices (/dev/usb/lp*)
- url: https://github.com/torvalds/linux/blob/master/drivers/usb/class/usblp.c
  why: the ieee1284_id sysfs attribute, and read/write behaviour
- url: https://docs.rs/nusb
  why: the later macOS path (pure-Rust USB)
- file: crates/f2000/src/lib.rs
  why: verify(); parse before sending
- file: crates/dtg-creator-max/src/app.rs
  why: printer_bar() (status ring, "Off-line" text, progress bar), the Print button tooltip
- doc: ../../print-file-format.md
  section: Layout (EJL preamble, remote mode): what the printer expects
```

### Files to create/modify

```bash
crates/printer-io/Cargo.toml             — CREATE — no deps on Linux
crates/printer-io/src/lib.rs             — CREATE — Printer, discover(), send()
crates/printer-io/src/linux.rs           — CREATE — sysfs discovery, /dev/usb/lp writer
crates/printer-io/src/network.rs         — CREATE — raw TCP 9100 sender
crates/printer-io/tests/fake_sysfs.rs    — CREATE — discovery and filtering tests
crates/dtg-creator-max/src/…             — MODIFY — printer bar status, confirm dialog, Print wiring (with PRD-006)
```

### Known gotchas

```
# A printer may block on write while it's busy. Write in chunks with a cancel flag; never
#   on the UI thread (PRD-006's worker).
# The device can vanish mid-job (unplugged). Report it as an error; never retry silently.
# The 1284 ID's MDL string is an assumption until read from the real printer. Keep the
#   match rule in one place.
# Don't add a udev rule or need root. The lp group is the documented route.
```

---

## Implementation blueprint

### Data models

```rust
pub struct Printer { pub id: PrinterId, pub model: String, pub port: Port }
pub enum Port { UsbLinux { dev: PathBuf }, Network { addr: SocketAddr } }
pub fn discover(sysfs_root: &Path) -> Vec<Printer>;
pub fn send(printer: &Printer, file: &Path, progress: impl FnMut(u64, u64), cancel: &AtomicBool) -> Result<(), SendError>;
```

### Tasks (in order)

```yaml
Task 1: Discovery from sysfs with an injectable root; fake-tree tests
Task 2: Chunked sender to a path (device, file or FIFO) with progress and cancel; tests
Task 3: Network sender (raw TCP 9100), tested against a local listener
Task 4: App: printer bar status, confirm dialog, "Send to file…", nozzle check (needs PRD-002)
Task 5 (hardware): nozzle check, then a built job; record the 1284 ID and any copies field in the decision log
```

### Integration points

```yaml
CONFIG:
  - remembered printer (PRD-006 prefs file)
DEPLOY:
  - none (document the lp group in README)
```

---

## Validation loop

### Level 1 — syntax & style

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```

### Level 2 — unit tests

```bash
cargo test -p printer-io
```

### Level 3 — integration

```bash
# no printer: dry run
./target/release/dtg-creator-max   # Send to file… → cmp with the source .prn (identical)
# printer (Linux): ls /dev/usb/lp*; cat /sys/class/usbmisc/lp0/device/ieee1284_id
```

### Level 4 — manual verification

- [ ] Hardware: the nozzle check prints cleanly. A built job prints. Unplugging mid-job
  gives a clear error.
- [ ] The confirm dialog and status text are clear and en-AU.

---

## Completion checklist

- [ ] All validation levels green (hardware steps when possible; if they're deferred, the
  PRD stays Blocked)
- [ ] README: the "printing on Linux" section (lp group)
- [ ] **Decision-log entry appended** (1284 ID string, network findings, copies behaviour)
- [ ] PRD moved to `completed/`; index regenerated; NEXT-SESSION updated

---

## Anti-patterns to avoid

- Don't pull in libusb or a CUPS dependency for something the kernel already does.
- Don't poll for the printer on a timer.
- Don't send unverified bytes to a printer.

---

## Confidence score: 6/10

Writing to `usblp` is standard and simple. The unknowns: exactly how the F2000 identifies
itself, whether it has network printing, and whether it needs anything beyond the file.
None of these can be answered without the printer.
