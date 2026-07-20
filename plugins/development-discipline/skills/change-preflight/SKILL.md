---
name: change-preflight
description: Use when preparing to edit a feature, bug fix, refactor, documentation or configuration update, packaging or release work, migration, operational change, or other substantive repository change; classify it before editing and explicitly decide all ten surfaces from repository or supplied evidence.
---

# Change preflight

Before editing, identify the complete change surface from repository evidence.
This is a short classification step, not a speculative implementation plan.

Start the answer with this exact record shape:

```text
Change classification: <feature | bug fix | refactor | docs/config | packaging/release | migration | operational change | mixed>

Affected surfaces
- Behavior: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Tests: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Documentation: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Configuration: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Packaging: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Release artifacts: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Migrations: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Operational startup: <applicable — evidence and effect | not applicable — evidence-backed reason>
- Evaluations: <applicable — evidence and effect | not applicable — evidence-backed reason>
- User workflows: <applicable — evidence and effect | not applicable — evidence-backed reason>
```

A preflight is incomplete unless it begins with the change classification and
gives an explicit `applicable` or `not applicable` decision for each of the ten
named surfaces below. Follow that record shape even in a short advisory answer.
Never substitute a generic checklist, a list of things to inspect later, or a
claim that evidence is unavailable when the request supplies repository facts.

## Precedence and triggers

Current user direction and repository-local instructions control the scope and
workflow. This skill fills gaps without overriding them.

Skip this skill when the request is purely conversational or read-only and no
repository edit is being prepared. Do not skip it merely because an edit is
small.

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
When secret-related configuration is relevant, inspect only non-secret evidence
such as schemas, templates, environment-variable names, and secret references.
Never open, quote, hash, or include populated secret files or secret values in
the preflight record.

In an advisory scenario where repository inspection is unavailable or
explicitly forbidden, use the repository facts stated in the request. Treat
those supplied facts as repository evidence, say that they are the available
evidence, and do not claim that an evidence-backed preflight is impossible.
Tie every row to a stated repository fact such as a named artifact, behavior,
workflow, or explicit absence. Naming that fact in the row is evidence; do not
replace it with a generic statement that the surface should be checked.

Classify the change that will actually be made, not every behavior mentioned in
its documentation. A documentation or checked-in example edit that describes
runtime behavior does not itself change that behavior or require behavior tests
when the runtime default, code, and shipped artifacts remain unchanged.

User workflows include operator workflows and workflows that consume guidance,
not only product UI steps. For an operational change, trace how operators
deploy, observe, and recover the service. For documentation or configuration
work, treat the setup workflow in which a user chooses, copies, or applies
documented configuration as applicable when that guidance changes.

When migrations are applicable, explicitly decide compatibility, rollback,
recovery, and backfill, even when one of those is not applicable. Include the
corresponding upgrade and recovery test or workflow evidence. When an
operational change is applicable, state the deploy, observe, and recover steps
in the relevant rows instead of compressing them into a general operations
claim.

Do not defer a migration decision with “determine later” or “if required.” Use
the available evidence to decide it now. If evidence is incomplete but the
ambiguity does not block the preflight, state the narrow assumption and mark
the decision applicable or not applicable; ask before editing only when the
answer would materially change the work.

Account for every surface below. Write all ten surface names explicitly, each
with `applicable` or `not applicable` and its evidence or evidence-backed
reason. Do not compress, combine, or silently omit rows, including in an
advisory answer.

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

For a migration, decide compatibility, rollback, recovery, and backfill
explicitly. Treat an on-demand migration performed separately in each existing
repository as the backfill strategy; the absence of one centralized batch job
does not make backfill inapplicable.
