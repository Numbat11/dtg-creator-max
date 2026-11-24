# Contributing to DTG Creator Max

Thanks for helping. A few things to know first.

## Licence of contributions

DTG Creator Max is released under the GNU General Public License, version 3 or later.
Numbat Web, the copyright holder, may also offer it under separate commercial licences.
That's how the project pays for itself while staying free for everyone.

So by submitting a contribution (code, documentation or data) you agree that:
1. it is your own work, or you have the right to submit it;
2. it may be distributed under the GPL-3.0-or-later; and
3. Numbat Web may also distribute it under other licence terms.

You keep the copyright in your contribution.

## What must never be committed

- Epson software or data: installers, libraries, colour profiles, halftone tables, firmware.
- Print files (`.prn`) or job files (`.gcr`) made by Epson's software.
- Anyone's artwork other than test images you made yourself.

Tests that need real print files read them from a private folder (see
`crates/f2000/tests/golden.rs`). Describe what you learned from such files in
`docs/print-file-format.md` instead of committing them.

## Before opening a pull request

```bash
cargo test
```
