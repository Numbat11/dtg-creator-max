# PRD-002 — Owner data import: the Epson files the printer needs, from the owner's own copy

**Status**: Planning
**Owner**: Numbat Web + Claude
**Created**: 2026-11-25 · **Updated**: 2026-11-25
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §4 (out of scope: shipping Epson data), §13 · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)
**Depends on**: PRD-001 (lints, dependency policy)

> Forward-looking INTENT. Once building starts, don't quietly rewrite it; record reality in
> the decision log.

### Core principles

1. **Context is King** · 2. **Validation Loops** · 3. **Information Dense** ·
4. **Progressive Success** · 5. **Global Rules** (`CLAUDE.md`), especially "never commit
Epson files".

---

## Goal

One small, well-tested library, plus an app screen, that finds the Epson data the SC-F2000
needs in the **owner's own** Garment Creator files. It verifies each item by checksum and
stores it in the user's data folder. The colour engine, the job builder and the sender can
then ask for it by name. Nothing Epson-owned is ever committed, bundled or downloaded by us.

## Why

- **Business value**: DTG Creator Max can only be open source if it never redistributes
  Epson data. Every F2000 owner has Garment Creator files already, so we read theirs.
- **Integration**: PRD-003 (colour) needs the garment colour profiles. PRD-005 (print files)
  needs the fixed 4,775-byte `EX` block that every print file carries. One import path
  serves both.
- **Problems solved**:
  - Scattered ad-hoc file lookups.
  - Silently using the wrong file version.
  - Accidentally committing Epson data. This module is the only place that touches it.

## What

### The data

| Item | Needed by | Where the owner has it |
|---|---|---|
| Printer setup block (4,775 bytes, identical in every job) | PRD-005 | Any print file Garment Creator saved, **or** the SC-F2000 model resources in a Garment Creator install |
| Garment colour profiles: dark with 1 white layer, dark with 2, light | PRD-003 | Garment Creator install, `assets/icc_profile/` |
| Transparency tables, colour and white (`A_COLOR`, `A_WHITE`) | PRD-003, if needed | Garment Creator install, SC-F2000 model folder |
| Nozzle-check print file, White Edition | PRD-007 (first hardware test, maintenance) | Garment Creator install, `assets/nzl_check/` |

### Behaviour

- **Sources the owner can point at:**
  1. **A print file** saved by Garment Creator. That gives the setup block only, so
     colour stays unavailable.
  2. **A Garment Creator folder:**
     - the macOS app bundle (`GarmentCreator.app`);
     - the Windows install folder;
     - or a copy of either on a USB stick. That's the Linux route: copy the folder from any
       Mac or PC once.
- Each item is checked against a **SHA-256 allow-list** of known versions (2.1.7 for now)
  kept in our source. Hashes are not Epson data and are safe to publish.
  - An unknown version is accepted with a visible warning and recorded as "unverified".
  - A file that fails to parse is refused.
- **Storage:** in the platform data folder. Nothing is written next to the source files.
  - Linux: `$XDG_DATA_HOME/dtg-creator-max/owner-data/`, falling back to
    `~/.local/share/…`.
  - macOS: `~/Library/Application Support/DTG Creator Max/owner-data/`.
  - Windows: `%APPDATA%\DTG Creator Max\owner-data\`.
- **App:**
  - A first-run banner: "To print, DTG Creator Max needs a few files from your own copy
    of Garment Creator."
  - An **Import…** button.
  - A status list (✓ verified · ⚠ unverified · missing) in an About/Setup window.
  - Print and Save print file name the missing item in their disabled tooltip.
- **CLI:** `f2000 owner-data import <path>` and `f2000 owner-data status`, for headless
  setups.

### Success criteria

- [ ] Importing the private reference print file stores the setup block. Its SHA-256 matches
  the allow-list, and status shows ✓.
- [ ] Importing a Garment Creator 2.1.7 folder (on the iMac, before it's wiped) stores all
  items, all ✓.
- [ ] A truncated or altered file is refused with a clear message. A different but valid
  version is stored as ⚠ unverified.
- [ ] No Epson bytes exist anywhere in the repository. A test scans every tracked file for
  a 4,775-byte window whose SHA-256 equals the setup block's known hash, and fails if it
  finds one. Only the hash is in the repo, never the bytes.
- [ ] The module has no dependency beyond `sha2`, and none at all for data paths.

---

## Context & references

### Must-read before implementing

```yaml
- url: https://specifications.freedesktop.org/basedir-spec/latest/
  why: XDG_DATA_HOME rules (Linux data folder)
- url: https://docs.rs/sha2
  why: SHA-256, pure Rust (RustCrypto, MIT/Apache)
- file: crates/f2000/src/prn.rs
  why: parse() and Item::Remote: the setup block is the remote command `EX` with 4,775 data bytes
- file: docs/print-file-format.md
  why: "Each page" section: where the setup block sits and why it's constant
- doc: ../../../CLAUDE.md
  section: Rules that bite (never commit Epson files)
```

### Current codebase structure

```bash
crates/f2000/src/prn.rs        # parse/write print files; Item::Remote { cmds }
crates/dtg-creator-max/src/    # app; files.rs opens user files
```

### Files to create/modify

```bash
crates/owner-data/Cargo.toml          — CREATE — deps: sha2, f2000
crates/owner-data/src/lib.rs          — CREATE — Item kinds, import(), status(), get()
crates/owner-data/src/known.rs        — CREATE — SHA-256 allow-list (hashes only)
crates/owner-data/src/paths.rs        — CREATE — platform data folder, std::env only
crates/f2000/src/main.rs              — MODIFY — `owner-data import|status` subcommands
crates/dtg-creator-max/src/app.rs     — MODIFY — first-run banner, Setup window, tooltips
crates/owner-data/tests/no_epson_bytes.rs — CREATE — scans `git ls-files` for the setup block by hash
```

### Known gotchas

```
# The setup block is the remote command EX whose data is 4,775 bytes. It appears once per
#   page and is identical every time. Take the first; check that all copies match.
# Paths in Garment Creator installs contain spaces ("DarkGarment WH1Layer.icm") and differ
#   between the macOS bundle and the Windows folder. Search by file name below the chosen
#   root instead of hard-coding layouts.
# Never log file contents. Log names, sizes and hashes only.
# Don't read files from the network or offer to download them.
```

---

## Implementation blueprint

### Data models

```rust
pub enum ItemKind { SetupBlock, ProfileDark1, ProfileDark2, ProfileLight, AlphaColour, AlphaWhite, NozzleCheckWhite }
pub enum Verified { Known { version: &'static str }, Unverified }
pub struct Stored { pub kind: ItemKind, pub path: PathBuf, pub sha256: [u8; 32], pub verified: Verified }
pub fn import(source: &Path) -> Result<Vec<Stored>, ImportError>;   // print file or folder
pub fn status() -> Vec<(ItemKind, Option<Stored>)>;
pub fn get(kind: ItemKind) -> Result<Vec<u8>, MissingError>;         // for PRD-003/005
```

### Tasks (in order)

```yaml
Task 1: Data folder paths (std only) with tests that set env vars in a temp HOME
Task 2: Hash allow-list: compute SHA-256 of each item from the private reference set and GC 2.1.7; record hashes only
Task 3: Import from a print file (setup block): parse with f2000::prn::parse; find EX with 4,775 bytes
Task 4: Import from a folder: walk, match names, verify, copy into the data folder atomically (write temp + rename)
Task 5: CLI subcommands in f2000 (owner-data import|status)
Task 6: App: banner, Setup window, disabled-button reasons
Task 7: Guard test: hash every 4,775-byte window of every tracked file (prefilter on
  the window length and the block's known size field); fail on a match with the allow-listed
  hash. It runs in normal `cargo test`, so CI enforces it.
```

### Integration points

```yaml
CONFIG:
  - env: XDG_DATA_HOME / HOME / APPDATA (read only, standard)
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
cargo test -p owner-data      # temp dirs; synthetic print file with a fake EX block → ⚠ unverified
```

### Level 3 — integration

```bash
./target/release/f2000 owner-data import "$DTG_REFERENCE/prn/capture-numbatweb.prn"
./target/release/f2000 owner-data status        # Expected: setup block ✓ (2.1.7)
./target/release/f2000 owner-data import /Applications/GarmentCreator.app   # iMac only, before the wipe
```

### Level 4 — manual verification

- [ ] First run shows the banner. After import, the Setup window lists every item ✓.
- [ ] The disabled Print tooltip names exactly what's missing.
- [ ] The guard test passes. Committing a print file into a scratch branch (with `git add -f`)
  makes it fail.

---

## Completion checklist

- [ ] All validation levels green
- [ ] `CLAUDE.md` Rules that bite: "Epson data only through `owner-data`"
- [ ] **Decision-log entry appended**
- [ ] PRD moved to `completed/`; index regenerated; NEXT-SESSION updated

---

## Anti-patterns to avoid

- Don't embed, download or "helpfully" bundle any Epson file, even in tests. Tests use
  synthetic stand-ins.
- Don't hard-code install layouts. Search by name under the root the user picked.
- Don't trust an unverified file silently. Show ⚠ and record it.

---

## Confidence score: 8/10

The setup block's location is proven. Profile files are plain ICC. The open question is
whether the transparency tables are needed at all; PRD-003 decides. The risk is other
Garment Creator versions with different hashes, handled as ⚠ unverified.
