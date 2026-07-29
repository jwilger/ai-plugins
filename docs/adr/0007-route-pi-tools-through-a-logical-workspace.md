# ADR-0007: Route Pi tools through a logical workspace

## Status

Accepted

## Date

2026-07-29

## Context

ADR-0006 attempted automatic worktree transitions by sending a private
extension slash command through `pi.sendUserMessage(..., { deliverAs:
"followUp" })`. In Pi 0.82.1 that API intentionally disables command handling,
so the private command reached the model as ordinary user chat and left queued
state behind. Session mutation remains available only to manually invoked
`ExtensionCommandContext` handlers.

The desired behavior does not require changing Pi's process or session cwd.
Other coding harnesses keep the host process in a coordination checkout and
route each operation explicitly into a linked worktree.

Cleanup also enumerated every ignored path into one `execFile` buffer. Large
`.dependencies` and `node_modules` trees could exceed `maxBuffer` before the
extension classified them as disposable.

## Decision

Keep Pi's host cwd and conversation in the coordination checkout. Persist one
branch-scoped logical-workspace entry in the Pi session. Creation and switching
validate an exact registered worktree and update this authority directly; they
do not send user messages, invoke private commands, replace sessions, or
relaunch Pi.

Use Pi's supported mutable `tool_call` event to resolve relative
read/write/edit/grep/find/ls paths against the logical workspace and replace
them with canonical contained absolute paths. Prefix every built-in bash call
with a shell-quoted `cd -- <logical-workspace> &&`. Route status, policy, guards,
review children, and component MCP calls through the same authority. Wrap
user-entered bash operations with the supported local-bash operations API so
they start there too.

Restore only the latest active-branch state whose canonical primary,
registration, checkout kind, and branch still match. Ordinary HEAD advancement
is valid. Rejected state falls back to the proven host checkout and reports a
typed warning.

Finish validates the clean linked logical workspace, persists primary routing,
then runs teardown and non-forced `git worktree remove`. The branch is never
deleted. If cleanup fails, primary routing remains active and the worktree is
preserved.

Inspect ignored state with an exclusion pathspec for generated roots and an
incremental, bounded first-result scanner. Check ignored `.envrc` separately,
allowing only the generated exact `use flake\n` content.

## Consequences

### Positive

- Model tools work in TUI and headless modes without unsupported Pi internals.
- No transition command can leak into model-visible chat or remain queued.
- Every relative built-in operation has an independently established workspace.
- Status, guards, delivery, review, and component tools agree on one authority.
- Large generated caches cannot exhaust ignored-file output buffers.
- Cleanup returns authority to primary before deleting a worktree and preserves
  its branch.

### Negative

- Pi resource discovery, project trust, context files, and the native prompt cwd
  remain anchored to the host checkout. A per-turn instruction makes the
  distinction explicit but does not reload those resources from the worktree.
- Bash cwd routing is protection against accidental relative operations, not a
  hostile-process sandbox. Deliberate absolute paths or subprocess behavior
  remain governed by the repository's trusted-owner threat model and existing
  observable command guards.
- Future built-in path-bearing tools must be admitted to the routing boundary
  deliberately.
- A Pi process launched by the superseded design from inside a linked worktree
  cannot remove its own configured host cwd safely. Finish preserves that
  worktree and requires one migration start from primary.

## Alternatives Considered

### Continue session replacement

Rejected because Pi exposes no supported deferred session-mutation API to model
callable tools, and injected follow-up commands are intentionally ordinary chat.

### Monkey-patch Pi internals

Rejected because prototype or runner patches are version-fragile and outside
the supported extension trust boundary.

### Require a manual slash command or relaunch

Rejected because it recreates the lifecycle interruption this feature exists to
remove and is unnecessary for logical routing.

## Related

- `docs/adr/0005-switch-pi-sessions-between-registered-worktrees.md`
- `docs/adr/0006-automate-pi-worktree-lifecycle.md`
- `docs/research/pi-worktree-session-switching.md`
- `plugins/development-system/skills/worktrees/SKILL.md`
