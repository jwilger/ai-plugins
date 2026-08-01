---
name: beads
description: Use for Beads task creation, deterministic ready-work selection, workflow molecules, dependencies, cross-worktree coordination, Tiber migration, and pushed-CI recovery when beads is enabled.
---

# Beads

Require `[features].beads = true` in `.development-system.toml`. Use the `bd`
CLI directly with `--json`; do not add an MCP layer when shell access exists.
Run `bd prime` for the installed version's canonical workflow guidance.

When this plugin is installed, it owns the harness lifecycle hooks. Do not add
duplicates through `bd setup` or user configuration: Claude Code runs
`bd prime --hook-json` at `SessionStart`; Codex runs `bd codex-hook` at
`SessionStart`, `PreCompact`, `PostCompact`, and `UserPromptSubmit`. Inspect
`development-system integrations --harness claude|codex` to verify the exact
contract without changing either harness.

Beads uses its embedded Dolt backend by default; normal operation does not need
a standalone `dolt` CLI. Initialize only from the primary checkout through the
development-system setup flow. Normal linked worktrees share that Beads
workspace. Commit task changes through Dolt and push them with `bd dolt push`
when a remote is configured; JSONL is migration/interchange data, not sync. If
`bd` is missing or outdated, use `development-system setup --enable beads` to
reopen the explicit user-global installation offer rather than adding Beads to
a project devshell. That offer must show the current status, pinned target,
`~/.local/bin` user-global destination, and no-sudo guarantee; install only
after approval, verify the pinned executable, keep a tools-only result free of
repository commits, and leave Beads unavailable when declined so the same
command can retry. When answering that unavailable-enabled regression, state
both outcomes rather than only giving the setup command: approval installs the
verified pinned executable with unchanged project policy; decline changes
nothing and the same command reopens the offer. Do not compress the offer to
"a pinned executable": enumerate `bd: unavailable`, the target version
(currently `1.1.2`), `~/.local/bin`, `user-global` scope, and `no sudo` in the
answer so the owner can audit the proposed user-scoped change.

For ready work, filter blocked issues first and choose deterministically by:
priority ascending, creation time ascending, then issue ID ascending. Claim the
selected issue atomically with `bd update <id> --claim` before implementation.
Use real `blocks`, `parent-child`, and `discovered-from` relationships; never
invent blocking edges merely to order unrelated backlog items.

For immediate work, pour the delivery formula named by `[beads].workflow` and
use its returned `new_epic_id` as the active work item:

```shell
bd mol pour <configured-workflow> --var work_item="<title>" \
  --var ci_workflow=ci.yml --json
```

When starting from an ordinary queued issue, claim it first, pour the delivery
molecule, then run `bd supersede <queued-id> --with <new-epic-id>` so context
and replacement remain explicit without maintaining duplicate active items.
Scope progression with `bd ready --mol <new-epic-id> --json`. Classify each
independently deliverable slice before pouring its formula:

- runtime or tooling behavior: `behavior-slice`;
- prose-only documentation: `documentation-slice`;
- CI workflow behavior whose real test is pushed CI: `ci-workflow-slice`;
- non-runtime configuration or metadata: `validation-only-slice`.

Pour each selected slice with its required variables. Wire real blocking edges
from the delivery molecule's `complete-change-slices` step to each slice's final
checkpoint, so delivery cannot advance early. For sequential slices, also make
the next slice's first step depend on the previous checkpoint. Use the same
pattern from `implement-scenario-steps` to each `bdd-step-cycle` refactor step,
and from `drive-implementation` to each required `unit-tdd-cycle` refactor step.
Use IDs from each pour command's `id_mapping`; dependency direction is
`bd dep add <dependent> <blocker>`.

A behavior slice writes and observes a failing executable acceptance test first,
implements scenario steps one at a time through `bdd-step-cycle`, and attaches a
`unit-tdd-cycle` whenever the current failure does not identify exactly one
small semantic unit. Run focused tests during micro-cycles. Run the complete
configured local test gate only after the behavior slice is green, then create
and push its checkpoint according to delivery policy.

Documentation and validation-only slices do not invent tests. They run only
causal format, schema, link, build, package, integrity, or generation checks.
A CI workflow slice runs available static validation, pushes an authorized test
SHA, and treats the resulting CI run as the behavioral test. Only terminal
success for the exact final authorized SHA closes the slice; an earlier test
SHA, another revision, or a queued, pending, running, canceled, or failed run
does not satisfy its gate.

An intentionally failing hosted run is intermediate evidence: iterate from its
run evidence, push a new authorized test SHA for each causal correction, and
close only after the exact final authorized SHA reaches terminal success.

Record commands, outcomes, commit SHAs, and run URLs in Beads comments before
closing workflow steps. Formula dependencies define the ready phase; do not
close a step merely to bypass its evidence contract.

For an unexpected terminal pushed-CI failure, create or pour one P0
`ci-recovery` issue labeled `development-system:ci-recovery`, claim it
atomically, and acquire the project merge slot. Every other session pauses
unrelated work. Release the hold and merge slot only after the exact replacement
run reaches terminal success. An intentionally failing run inside the active
`ci-workflow-slice` is related test work, not a separate recovery incident.

To retire an existing Tiber board, run
`development-system migrate-tiber-to-beads --dry-run` from the primary checkout,
review the count/export, then rerun with `--apply --yes` and optionally `--push`
for the Dolt remote. The migration preserves historical states, labels,
acceptance criteria, notes, dependency links, board order metadata, and original
IDs as external references.
