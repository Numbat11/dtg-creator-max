# 🐰 BUGS BUNNY — Issue Tracker: DTG Creator Max

> *"What's up, Doc?"* — Bugs Bunny
> A running log of bugs and oddities discovered mid-session that are **out-of-context to
> fix immediately**. Come back here between sessions or when work in the relevant area
> resumes.

---

## How to use

- Add issues here the moment you spot them during unrelated work — don't derail the
  current PRD, don't trust yourself to remember.
- Number sequentially: `BUG-NNN`. Numbers are never reused.
- Include: symptom, suspected cause, relevant file paths, discovered date, severity.
- Mark ✅ FIXED when resolved (keep the entry for history — it's the trap-record for
  future sessions). If a fix ships as part of a PRD, cross-reference it and add the
  decision-log entry as usual.
- Promote to a full PRD if the fix turns out to need real design work.
- Newest entries at the top of each section.

---

## 🔴 Open issues

<!-- ENTRY TEMPLATE — copy below this comment.

### BUG-NNN — {{One-line symptom}}
**Discovered**: YYYY-MM-DD ({{session / what you were doing when it surfaced}})
**Severity**: High | Medium | Low — {{who it bites and how hard}}
**Status**: 🔴 OPEN | ✅ FIXED ({{when}})

{{Symptom in concrete terms — what was observed vs expected. Suspected cause if you have
one. How to reproduce, if known. What was checked/ruled out.}}

**Files**: `{{path/to/suspect.ts}}`

-->

### BUG-001 — Opening a large print file freezes the window for about a second
**Discovered**: 2026-11-25 (testing the desktop app's first version with 35–60 MB `.prn` files)
**Severity**: Low — only print files, only a short pause, no data at risk
**Status**: 🔴 OPEN

`files::open_prn` parses the print file, decodes every page to dot planes and builds the
preview on the UI thread, so the window stops responding until it finishes. It took about
a second in release builds on the 2014 iMac; debug builds are much slower. The fix is to
decode on a background thread and show "Opening…" in the status line. That's worth doing
when PRD-004 adds the save and print flows, which need the same pattern.

**Files**: `crates/dtg-creator-max/src/files.rs`, `crates/dtg-creator-max/src/app.rs`

---

## ✅ Fixed

*(entries move here — or stay in place with status flipped — when resolved)*
