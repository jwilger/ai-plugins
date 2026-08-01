# ADR 0009: Retire Pi and adopt native harness workflows

## Status

Accepted

## Date

2026-07-31

## Supersedes

This decision supersedes the Pi-primary portions of:

- [`0003-make-pi-the-primary-development-system-adapter.md`](0003-make-pi-the-primary-development-system-adapter.md)
- [`0004-bootstrap-linked-worktrees-with-semantic-pi-tools.md`](0004-bootstrap-linked-worktrees-with-semantic-pi-tools.md)
- [`0005-switch-pi-sessions-between-registered-worktrees.md`](0005-switch-pi-sessions-between-registered-worktrees.md)
- [`0006-automate-pi-worktree-lifecycle.md`](0006-automate-pi-worktree-lifecycle.md)
- [`0007-route-pi-tools-through-a-logical-workspace.md`](0007-route-pi-tools-through-a-logical-workspace.md)
- [`0008-stream-redacted-review-child-progress.md`](0008-stream-redacted-review-child-progress.md)

It does **not** supersede
[`0005-use-beads-and-dolt-for-task-workflow-state.md`](0005-use-beads-and-dolt-for-task-workflow-state.md).
Beads remains the task and workflow authority.

## Context

The former primary adapter added a package, extension runtime, custom UI,
event-intercepted guards, logical-workspace routing, goal state, package
canaries, and harness-specific authentication handling. Those mechanisms were
valuable only while that harness was a supported product surface. Keeping them
would make Codex and Claude Code secondary implementations and retain a large
runtime, validation, and documentation burden without a current user need.

The enduring value is not the retired adapter. It is the shared engineering
workflow: Beads-backed task coordination, evidence-oriented delivery formulas,
repository policy, skills, review support, and disciplined use of linked
worktrees when concurrent mutable work genuinely needs isolation.

Codex and Claude Code already provide their own session, goal, tool, and
worktree experiences. A plugin should extend those native experiences rather
than simulate a third harness inside them.

## Decision

Codex is the primary development-system harness. Claude Code remains a
supported peer surface. Both load the same shared skills and repository policy;
their marketplace manifests, hooks, and adapter metadata remain explicit and
version-synchronized.

Retire the Pi package and every Pi-only runtime contract, including:

- trusted local-TUI setup and status UI;
- event-intercepted write, shell, delivery, and worktree guards;
- session-persistent logical-workspace routing;
- the package-owned goal command, goal store, and continuation driver;
- Pi-specific review-child process projection;
- package loading, authentication, guard, and provider canaries; and
- Pi-specific installation, package-release, and compatibility documentation.

These capabilities are intentionally retired, not reimplemented as a hidden
cross-harness abstraction. The active harness owns its native goal handling and
the lifecycle of any linked worktree.

Retain the following workflow rules:

- Questions and read-only investigation need neither a ticket nor a worktree.
- Planned mutable delivery work uses Beads as the sole task and workflow
  authority.
- One mutable ticket may use its existing checkout.
- A linked worktree is used only to isolate distinct mutable tickets running
  concurrently across sessions, agents, or subagents.
- Repository-specific post-checkout bootstrap and optional teardown remain
  available around a linked worktree, but cleanup is housekeeping rather than a
  delivery requirement.

Beads remains direct CLI and plugin-owned-hook integration. Do not replace it
with an MCP task bridge or a second tracker. Keep its Dolt-backed formulas,
atomic claims, and CI-recovery workflow as established by ADR 0005.

Optional integrations are owner-operated. The read-only
`development-system integrations --harness <all|codex|claude>` guide describes
the handoff: Beads hooks are plugin-owned; Context7 uses official marketplace
installation; Claude Code Hindsight uses its marketplace installation; and
Codex Hindsight uses its official interactive installer or manual
configuration. The plugin never embeds keys or retention configuration. Owners
supply a Context7 key where applicable and make explicit Hindsight
retention/memory-bank decisions before enabling it.

## Consequences

- The active product surface is smaller and easier to reason about: Codex first,
  Claude Code supported, and no third runtime to install, authenticate, or
  validate.
- Documentation, eval selection, and CI focus on the two supported harnesses.
- Native harness behavior is authoritative for goals, session continuation, and
  worktree lifecycle; repository scripts retain only repository-specific
  bootstrap and teardown responsibilities.
- Historical Pi design documents and ADRs remain intact as evidence. They do
  not define current behavior.
- Some deterministic interception and UI guarantees formerly offered by the
  retired adapter no longer exist. The product deliberately relies on native
  harness capabilities, plugin hooks where available, repository policy, and
  normal review and verification instead.

## Alternatives considered

### Keep Pi as a supported third harness

Rejected. The maintenance cost of the package, extension APIs, guard surface,
auth handling, and dedicated canaries is not proportionate to its current
value.

### Port the Pi-only UI and logical workspace into Codex or Claude Code

Rejected. This would recreate the retired abstraction and conflict with native
harness lifecycle ownership. The workflow requirements are met with shared
skills, Beads, native goals, and narrowly scoped repository bootstrap/teardown
support.

### Replace Beads with harness-native tickets or a plugin MCP service

Rejected. It would undo ADR 0005's single durable task authority and duplicate
Beads' dependency, formula, claim, and Dolt behavior.

## Related

- [`../pi-extension-prd.md`](../pi-extension-prd.md) — withdrawn historical
  proposal
- [`0005-use-beads-and-dolt-for-task-workflow-state.md`](0005-use-beads-and-dolt-for-task-workflow-state.md)
- [`../../plugins/development-system/README.md`](../../plugins/development-system/README.md)
