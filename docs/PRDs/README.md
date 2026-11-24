# PRDs — DTG Creator Max

Product Requirement Documents: detailed, AI-implementable plans for every feature.
This is the **hybrid system** — popyalar's PRD structure + numbat-web's spec-driven
lifecycle (status headers, append-only decision log, record-reality rule).

## Directory structure

```
PRDs/
├── README.md            # this file
├── 00_decision_log.md   # append-only record of what was ACTUALLY built — never edited
├── templates/
│   ├── prd_template.md  # full — complex features
│   └── prd_simple.md    # simple — small features, clear fixes
├── pending/             # written, not started
├── active/              # currently being implemented
└── completed/           # implemented, validated, recorded
```

`docs/KICKOFF-BRIEF.md` is **PRD-000** — the master planning doc every PRD traces back to.

## Which template

**Full PRD** when: multiple services/files touched · complex business logic · new
patterns introduced · external API integrations · anything with data-model changes.

**Simple PRD** when: single-file changes · bug fixes with a clear solution · minor
enhancements.

## Lifecycle (plan → build → record)

```
1. PLAN    Copy a template → PRD-NNN-feature-name.md in pending/.
           Next free number; numbers are NEVER reused. Status: Planning.
2. BUILD   Move to active/, Status: In progress. Update the status header +
           any scope changes as you go — but once building starts, the PRD is
           forward-looking INTENT: don't quietly rewrite its history.
3. VALIDATE Run the PRD's own validation loop until green.
4. RECORD  Append an entry to 00_decision_log.md describing what ACTUALLY
           shipped and why (especially where it differs from the plan).
           Move the PRD to completed/, Status: Complete.
           Distil durable conclusions into Claude memory / CLAUDE.md.
5. SUPERSEDE If a later PRD replaces this one, mark the old one Superseded
           with a pointer — never delete it.
```

Statuses: `Planning → In progress → Built/Complete → Superseded` (+ `Blocked` if waiting
on the client or a dependency — say on what).

## Best practices

- **Context is king** — include ALL docs, file references, examples and gotchas the
  implementing agent needs. Reference existing patterns by path.
- **Be specific** — vague requirements produce wrong implementations.
- **Validation first** — write the validation steps before the implementation steps.
- **Define success** — measurable criteria, checkbox form.
- **Update as you learn** — implementation reveals information; capture it (PRD while
  active, decision log once shipped).
- **PII/secrets never in git** — reference env-var *names* and CRM job links only.
- **Update the relevant PRD before each commit** that touches its feature.
