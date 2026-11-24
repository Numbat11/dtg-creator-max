# PRD-NNN — [Feature Name]

**Status**: Planning | In progress | Blocked (on what) | Complete | Superseded (by PRD-NNN)
**Owner**: [Name] + Claude
**Created**: YYYY-MM-DD · **Updated**: YYYY-MM-DD
**Kickoff brief**: [../../KICKOFF-BRIEF.md](../../KICKOFF-BRIEF.md) §[section] · **Decision log**: record reality in [00_decision_log.md](../00_decision_log.md)

> Status flow: Planning → In progress → Complete → Superseded. Update the header as it moves.
> This doc is forward-looking INTENT — once building starts, don't quietly rewrite it.
> Record what actually ships in the decision log.

### Core principles

1. **Context is King** — include ALL necessary documentation, examples, and caveats
2. **Validation Loops** — provide executable tests the agent can run and fix
3. **Information Dense** — use keywords and patterns from THIS codebase
4. **Progressive Success** — start simple, validate, then enhance
5. **Global Rules** — follow everything in `CLAUDE.md` (incl. client copy rules)

---

## Goal

[What needs to be built — be specific about the end state]

## Why

- **Business value**: [client impact; trace to a kickoff-brief goal]
- **Integration**: [how this fits with existing features]
- **Problems solved**: [what pain points this addresses, for whom]

## What

[User-visible behaviour and technical requirements]

### Success criteria

- [ ] [Specific measurable outcome 1]
- [ ] [Specific measurable outcome 2]

---

## Context & references

### Must-read before implementing

```yaml
- url: [official docs URL]
  why: [specific sections needed]

- file: [path/to/existing/pattern]
  why: [pattern to follow / gotcha to avoid]

- doc: [internal doc, e.g. ../../KICKOFF-BRIEF.md §8]
  section: [relevant section]
```

### Current codebase structure

```bash
# Output of: tree -L 2 (relevant slice only)
```

### Files to create/modify

```bash
# path — CREATE|MODIFY — responsibility
```

### Known gotchas

```
# THIS project's patterns and pitfalls — framework quirks, client copy rules,
# deploy traps, library versions. Pull recurring ones from CLAUDE.md.
```

---

## Implementation blueprint

### Data models

```ts
// interfaces / schemas / migrations this feature needs
```

### Tasks (in order)

```yaml
Task 1: [name]
  Action: CREATE|MODIFY
  Files:
    - path/to/file
  Details:
    - [specific step]
    - [pattern to mirror, by path]

Task 2: ...
```

### Pseudocode (only for non-obvious logic)

```
// PATTERN: follow [x] from [file]
// GOTCHA: [caveat]
```

### Integration points

```yaml
DATABASE:
  - migration: [...]
API:
  - endpoint: [METHOD /path] · auth: [...]
CONFIG:
  - env: [VAR_NAMES only — values live in Coolify/host env]
DEPLOY:
  - [any change to the deploy shape — static→Node, new volume, new resource]
```

---

## Validation loop

### Level 1 — syntax & style

```bash
# lint / typecheck commands for this stack. Expected: no errors.
```

### Level 2 — unit tests

```bash
# test commands. Expected: pass. Never mock just to pass.
```

### Level 3 — integration

```bash
# run the app, curl the endpoint / drive the flow. Expected response described.
```

### Level 4 — manual verification

- [ ] Works in browser (desktop + mobile viewport)
- [ ] Error / empty / loading states behave
- [ ] `prefers-reduced-motion` respected (if motion involved)
- [ ] Accessibility spot-check (keyboard, contrast, labels)
- [ ] Client copy rules obeyed in all visitor-facing text

---

## Completion checklist

- [ ] All validation levels green
- [ ] No console errors
- [ ] Docs updated (CLAUDE.md if a durable rule emerged)
- [ ] **Decision-log entry appended** (what actually shipped, why, divergences)
- [ ] Related `BUG-NNN` entries in BUGS_BUNNY.md closed; new stray bugs logged there
- [ ] PRD moved to `completed/`, status updated
- [ ] Docs index regenerated if docs changed (`npm run generate:docs-index`)
- [ ] NEXT-SESSION.md updated if deploy state changed

---

## Anti-patterns to avoid

- Don't create new patterns when existing ones work
- Don't skip validation because "it should work"
- Don't ignore failing tests — fix them
- Don't hardcode values that should be config
- Don't catch all exceptions — be specific
- Don't over-engineer — keep it simple
- Don't put PII or secret values in this file

---

## Confidence score: [X/10]

[What's certain, what's assumed, what could force a re-plan]
