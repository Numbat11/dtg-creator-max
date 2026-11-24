# PRD-001 — Engineering foundations: CI, lints, dependency policy and a memory budget

**Status**: Planning
**Owner**: Numbat Web + Claude
**Created**: 2026-11-25 · **Updated**: 2026-11-25
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §6, §11, §13 · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)

> Status flow: Planning → In progress → Complete → Superseded. Update the header as it moves.
> This doc is forward-looking INTENT. Once building starts, don't quietly rewrite it;
> record what actually ships in the decision log.

### Core principles

1. **Context is King**: include all necessary documentation, examples and caveats.
2. **Validation Loops**: provide executable tests the agent can run and fix.
3. **Information Dense**: use keywords and patterns from this codebase.
4. **Progressive Success**: start simple, validate, then enhance.
5. **Global Rules**: follow everything in `CLAUDE.md`.

---

## Goal

Every change to DTG Creator Max is checked automatically on Linux, macOS and Windows, and
the code follows written, enforced rules. Those rules keep it small, safe, reproducible and
efficient for years. Every later PRD inherits them.

## Why

- **Business value**: an open-source printer tool lives or dies on trust and longevity.
  Outside contributors, and future sessions, need guard rails that catch mistakes before
  a printer does.
- **Integration**: no CI exists today. Formatting, lint and test rules live only in
  people's heads. The golden tests need a private reference set, so CI must work without
  it.
- **Problems solved**:
  - Regressions slipping into `main`.
  - Dependency creep, including licences that clash with the GPL and C toolchains that
    break builds.
  - Memory use that rules out older machines: today about 255 MB per decoded job.

## What

### The sustainability rules (the standing policy every PRD follows)

1. **Long-lived code.**
   - Small crates with one job each. Pure, deterministic functions for anything numeric.
   - The tests are the specification: golden tests against real printer output, plus
     property tests for invariants.
   - No `unsafe` in our crates.
2. **A small, pure-Rust dependency tree.**
   - Library crates (`f2000`, the engine) take a dependency only when it replaces
     substantial, hard-to-get-right code.
   - No C toolchains in the default build.
   - Every licence must be GPL-3.0-compatible, checked by `cargo-deny`.
3. **Hardware efficiency.**
   - Must run well on 10-year-old computers. The reference machine is a 2014 iMac.
   - Memory is bounded by streaming in bands, not whole pages.
   - Parallelism uses `std::thread::scope`, with no runtime.
   - Graphics need only OpenGL.
4. **Energy efficiency.**
   - No background polling or busy loops. The UI repaints only on input or progress.
   - No telemetry, no network access unless the user sends to a network printer.
   - CI caches builds.
5. **Reproducible and auditable releases.**
   - `Cargo.lock` is committed and the toolchain pinned in `rust-toolchain.toml`.
   - Binaries are built with `cargo auditable`, so their dependency list is embedded.
6. **Accessible.** AccessKit stays on. Every control has a label, and disabled controls
   say why.

### Technical requirements

- `rust-toolchain.toml` pins the stable toolchain. `rust-version = "1.95"` in
  `[workspace.package]`, which is eframe 0.37's MSRV.
- `[workspace.lints]`:
  - `unsafe_code = "forbid"`;
  - clippy `all` as warnings, plus a short list of `pedantic` lints that earn their keep;
  - `missing_docs = "warn"` for library crates.
- Every crate opts in with `[lints] workspace = true`.
- `rustfmt.toml` with `max_width = 120`, matching the current style. One formatting
  commit, done on its own.
- `deny.toml`:
  - licence allow-list: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, MPL-2.0
    and GPL-3.0-or-later;
  - advisories deny;
  - duplicate versions warn;
  - unknown registries and git sources deny.
- GitHub Actions `ci.yml`, on push and pull request:
  - fmt check and clippy `-D warnings` on Ubuntu;
  - `cargo test` on `ubuntu-latest`, `macos-latest` and `windows-latest`;
  - `cargo deny check`;
  - docs-index verify (`npm ci && npm run verify:docs-index`).
  - Cache with `Swatinem/rust-cache`. Pin actions by commit SHA.
- Dependabot for `cargo` and `github-actions`, monthly, grouped. Monthly saves CI energy.
- Release profile: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`. Smaller
  downloads, faster cold start.
- **Memory budget.**
  - Add a band-streaming API to `f2000::raster`, a `BandSource` trait. `encode_raster` can
    then take bands on demand instead of a `Page` holding full planes.
  - The existing `Page`-based functions stay, for verification.
  - Target: encoding a full 14 × 16 in job needs **< 64 MB** of working memory.
- `CHANGELOG.md` in Keep a Changelog format, plus a `SECURITY.md` saying how to report
  issues.

### Success criteria

- [ ] CI is green on Linux, macOS and Windows for `main` and for every PR.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `cargo deny check` passes. No dependency is under a GPL-incompatible licence.
- [ ] Neither crate contains `unsafe`; the lint forbids it.
- [ ] Golden tests still pass locally with the reference set and skip cleanly in CI.
- [ ] Encoding a decoded 50 MB reference job through the streaming API uses < 64 MB peak
  RSS (measured with `/usr/bin/time -l` on macOS or `-v` on Linux) and stays byte-identical.
- [ ] `CHANGELOG.md`, `SECURITY.md`, `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml` and
  `.github/` exist.

---

## Context & references

### Must-read before implementing

```yaml
- url: https://doc.rust-lang.org/cargo/reference/workspaces.html#the-lints-table
  why: workspace-level lints, and how crates opt in
- url: https://embarkstudios.github.io/cargo-deny/
  why: deny.toml sections (licenses, advisories, bans, sources)
- url: https://github.com/rust-secure-code/cargo-auditable
  why: embedding the dependency list in release binaries
- url: https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuration-options-for-the-dependabot.yml-file
  why: grouped monthly updates
- file: crates/f2000/src/raster.rs
  why: encode_raster / garment_creator_strip / Plane: the code the streaming API wraps
- file: crates/f2000/tests/golden.rs
  why: how the reference set is found ($DTG_REFERENCE, ./reference) and skipped
- doc: ../../../CLAUDE.md
  section: Rules that bite
```

### Current codebase structure

```bash
Cargo.toml                 # workspace: members crates/*, profiles
crates/f2000/              # print-file library + f2000 CLI (deps: miniz_oxide)
crates/dtg-creator-max/    # the app (eframe 0.37 glow, rfd, image)
docs/  scripts/  .githooks/  package.json   # docs + docs-index tooling
```

### Files to create/modify

```bash
rust-toolchain.toml                  — CREATE — pin stable
rustfmt.toml                         — CREATE — max_width = 120
deny.toml                            — CREATE — licence/advisory/source policy
Cargo.toml                           — MODIFY — rust-version, [workspace.lints], release profile
crates/*/Cargo.toml                  — MODIFY — [lints] workspace = true
crates/f2000/src/raster.rs           — MODIFY — BandSource trait + streaming encode
.github/workflows/ci.yml             — CREATE — fmt, clippy, test ×3 OS, deny, docs index
.github/dependabot.yml               — CREATE — cargo + actions, monthly, grouped
CHANGELOG.md  SECURITY.md            — CREATE
```

### Known gotchas

```
# CI has no reference set. Golden tests must keep skipping cleanly, never failing.
# rfd's Linux build uses the XDG portal (no GTK headers). ubuntu-latest needs no extra apt
#   packages for building. Running the app in CI needs a display; don't, use unit tests.
# egui 0.37: keep the glow renderer (wgpu fails on old GPUs; see CLAUDE.md).
# cargo-deny's own MSRV (1.95) can differ from ours. Install it as a prebuilt binary in
#   CI, not with `cargo install`.
# The pre-commit hook needs node_modules. CI runs `npm ci` before verify:docs-index.
```

---

## Implementation blueprint

### Data models

```rust
/// Supplies one band (up to BAND_LINES rows) of one channel at a time, so a job can be
/// encoded without holding whole-page planes.
pub trait BandSource {
    fn format(&self) -> PageFormat;
    fn bits(&self) -> u8;
    fn channels(&self) -> &[u8];
    /// Fill `out` (lines × width_bytes) with rows y..y+lines of `channel`.
    fn band(&mut self, channel: u8, y: usize, lines: usize, out: &mut [u8]);
}
impl BandSource for Page { /* reads from the full planes: keeps rebuild() working */ }
pub fn encode_bands(src: &mut impl BandSource) -> Vec<Item>;
```

### Tasks (in order)

```yaml
Task 1: Toolchain and lint policy
  Action: CREATE/MODIFY
  Files: rust-toolchain.toml, rustfmt.toml, Cargo.toml, crates/*/Cargo.toml
  Details:
    - Pin stable. Set rust-version = "1.95". Add [workspace.lints] and opt crates in.
    - Fix any lint findings in their own commit, then a separate `cargo fmt` commit.
Task 2: Dependency policy
  Action: CREATE
  Files: deny.toml
  Details:
    - Licence allow-list as above. Check that GPL-3.0-or-later is allowed for our own crates.
Task 3: CI
  Action: CREATE
  Files: .github/workflows/ci.yml, .github/dependabot.yml
  Details:
    - Job "lint" (ubuntu): fmt --check, clippy -D warnings, cargo deny, docs-index verify.
    - Job "test" (matrix ubuntu/macos/windows): cargo test --workspace --locked.
    - Swatinem/rust-cache. Actions pinned by SHA with a version comment.
Task 4: Streaming encoder
  Action: MODIFY
  Files: crates/f2000/src/raster.rs, crates/f2000/src/lib.rs
  Details:
    - Add BandSource and encode_bands. encode_raster(page) becomes encode_bands(page).
    - garment_creator_strip takes a band slice instead of a &Plane.
    - Keep crop rules and output byte-identical (golden tests prove it).
Task 5: Release profile and project files
  Action: MODIFY/CREATE
  Files: Cargo.toml, CHANGELOG.md, SECURITY.md
```

### Pseudocode (only for non-obvious logic)

```
// GOTCHA: garment_creator_strip pads to an even width only when it compresses. A raw
//   (mode 4) strip keeps its exact width. Keep this exactly; it's golden-tested on 88 files.
// PATTERN: crop() from raster.rs decides (start byte, bytes needed) per band and channel.
```

### Integration points

```yaml
CONFIG:
  - env: DTG_REFERENCE (tests only; unset in CI)
DEPLOY:
  - none (CI only)
```

---

## Validation loop

### Level 1 — syntax & style

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check
npm run verify:docs-index
```

### Level 2 — unit tests

```bash
cargo test --workspace --locked          # with the private reference set present locally
DTG_REFERENCE=/nonexistent cargo test --workspace   # as CI sees it: golden tests skip
```

### Level 3 — integration

```bash
cargo build --release
/usr/bin/time -l ./target/release/f2000 rebuild <ref>/prn/capture-numbatweb.prn /tmp/out.prn   # macOS; -v on Linux
# Expected: "identical to the input", and maximum resident set size < 64 MB
```

### Level 4 — manual verification

- [ ] A PR shows the three-OS CI matrix green.
- [ ] A deliberately bad PR (clippy warning, unformatted file, GPL-incompatible dependency)
  fails CI with a clear message.
- [ ] The app still opens and draws on the iMac (`--screenshot` check).

---

## Completion checklist

- [ ] All validation levels green
- [ ] Docs updated: `CLAUDE.md` gets the sustainability rules summary and the CI commands
- [ ] **Decision-log entry appended**
- [ ] PRD moved to `completed/`, status updated
- [ ] Docs index regenerated (`npm run generate:docs-index`)
- [ ] NEXT-SESSION.md updated

---

## Anti-patterns to avoid

- Don't add a dependency to save ten lines. Do add one to avoid reimplementing a hard,
  well-tested thing.
- Don't make CI depend on the private reference set, or on network services beyond
  crates.io and GitHub.
- Don't run scheduled CI more than needed: on push and PR, plus monthly Dependabot.
- Don't change raster output while refactoring. The golden tests are the referee.

---

## Confidence score: 8/10

The tooling is standard and well documented. The streaming refactor is the only real code
change, and 88 golden files guard it. The risk is clippy `pedantic` noise. Keep the
selected list short and justified.
