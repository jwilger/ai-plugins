---
name: change-preflight
description: Use before editing for a feature, bug fix, refactor, documentation or configuration update, packaging or release work, migration, operational change, or other substantive repository change to identify every affected project surface and its evidence.
---

# Change preflight

Before editing, identify the complete change surface from repository evidence.
This is a short classification step, not a speculative implementation plan.

## Precedence and triggers

Current user direction and repository-local instructions control the scope and
workflow. This skill fills gaps without overriding them.

Run the full preflight for features, bug fixes, refactors, packaging or release
work, migrations, operational changes, and any request whose effects may cross
files or tools. A genuinely narrow documentation or metadata-only edit may use
the same output with irrelevant rows marked `not applicable`; give a concrete
reason instead of silently skipping them. If evidence shows a supposedly narrow
change affects another surface, use the full preflight.

## Evidence pass

Read the request, repository instructions, nearby implementation, existing
tests, build and package metadata, release automation, migrations, startup
paths, evals, and user-facing workflows that could govern the change. Prefer
repository facts such as paths, commands, manifests, schemas, and workflow
files. Do not invent obligations merely because a surface exists in the table.

Before the first edit, emit this concise record:

```text
Change classification: <feature | bug fix | refactor | docs/config | packaging/release | migration | operational | mixed>

Affected surfaces
- <surface>: applicable — <repository evidence and expected effect>
- <surface>: not applicable — <evidence-backed reason>
```

Account for every surface below. Combine adjacent rows when that stays clear,
but do not omit a row.

| Surface             | Questions grounded in repository evidence                       |
| ------------------- | --------------------------------------------------------------- |
| Behavior            | What externally observable behavior or invariant changes?       |
| Tests               | Which focused, regression, integration, or mutation tests?      |
| Documentation       | Which user, operator, contributor, or policy guidance changes?  |
| Configuration       | Which schemas, defaults, manifests, flags, or secrets shape it? |
| Packaging           | Which built, bundled, generated, or checksummed artifacts?      |
| Release artifacts   | Which versions, catalogs, changelogs, or publication metadata?  |
| Migrations          | Which data, schema, compatibility, rollback, or backfill path?  |
| Operational startup | Which hooks, boot, deployment, health, or recovery behavior?    |
| Evaluations         | Which agent behavior fixtures, benchmarks, or quality gates?    |
| User workflows      | Which end-to-end task or delivery sequence changes?             |

End with the smallest evidence-backed set of applicable surfaces. Resolve a
material ambiguity before editing only when the answer would change the work;
otherwise record the assumption and continue.
