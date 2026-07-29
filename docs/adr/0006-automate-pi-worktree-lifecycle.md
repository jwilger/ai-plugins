# ADR-0006: Automate the Pi worktree lifecycle

## Status

Superseded by ADR-0007

## Date

2026-07-29

## Context

ADR-0005 added safe in-process session replacement, but required the user to
enter and confirm a slash command after a model had already created or selected
a worktree. Pi restricts `switchSession()` to command contexts, but its public
extension API also permits a model-callable tool to queue an extension command
as a follow-up message. The extra handoff was therefore development-system
policy rather than a Pi safety requirement.

The extension also treated every unclassified primary-checkout shell command as
mutation. That blocked ordinary Git inspection, pipelines, test commands, and
builds even though repository hooks and narrower extension guards already keep
commits and direct tracked writes out of the coordination checkout.

Finally, the lifecycle had creation and switching but no semantic completion
operation. Finished worktrees accumulated unless users remembered a
repository-specific removal command.

## Decision

Add model-callable `development_system_worktree_switch` and
`development_system_worktree_finish` tools. Each queues a private, one-time
follow-up extension command. The command runs after the current response settles
and uses the supported command-context session-replacement API. Automatic
requests revalidate the registered path, branch, and HEAD and do not require a
second confirmation. Manual slash commands retain confirmation.

`development_system_worktree_create` queues the same automatic switch in local
TUI mode. Headless modes retain the explicit relaunch command because they
cannot replace the interactive runtime.

Resolve repository identity before policy and always read
`.development-system.toml` from the canonical primary checkout. Every linked
worktree therefore inherits one authoritative policy even when the file is not
present in that checkout.

Permit unknown primary-checkout shell commands so exploration, tests, and builds
remain usable. Continue blocking direct write/edit tools, redirection, obvious
filesystem mutation, and Git index, history, branch, and worktree mutation.
Delivery and destructive-operation guards remain independent.

The finish operation requires a clean, registered, identity-stable linked
worktree. It first replaces the Pi session into the primary checkout, then runs
an executable repository `scripts/worktree-teardown.sh` when present and invokes
non-forced `git worktree remove`. It never deletes the branch. Dirty state,
detached or changed identities, ignored state outside known generated cache
paths, teardown failures, and removal failures preserve the worktree and report
an actionable error.

## Consequences

### Positive

- Creating or selecting a worktree no longer interrupts the conversation with a
  user-operated handoff.
- Linked runtimes consistently recognize primary-checkout configuration.
- Coordination checkouts remain useful for inspection and verification while
  tracked changes and commits stay isolated.
- Verified work has an explicit, safe cleanup path that includes project-owned
  teardown and preserves branches.

### Negative

- A model tool can initiate conversation transfer without a second user prompt.
  This is intentional for a trusted local owner workflow and is bounded by a
  one-time token plus Git identity revalidation.
- Unknown shell commands may have incidental side effects such as build caches.
  The extension no longer pretends it can infer arbitrary program behavior.
- Cleanup may return the session to primary but preserve the worktree when
  teardown or removal fails; callers must resolve the reported failure.
- Headless callers still need a replacement process.

## Alternatives Considered

### Keep the user-entered switch command

Rejected because it adds recurring friction without adding a Pi-enforced safety
boundary. The same extension can queue its own command through the public API.

### Continue failing closed on every unclassified shell command

Rejected because arbitrary command semantics cannot be proven from shell text,
and the policy prevented normal coordination work. Narrow, observable mutation
boundaries and repository Git hooks better match the intended threat model.

### Remove worktrees on session shutdown

Rejected because exiting or reloading a session does not mean delivery is
complete. Cleanup must be an explicit lifecycle action after verification and
must reject dirty worktrees.

### Force removal or delete the branch

Rejected because cleanup must preserve recoverable user state. Non-forced
worktree removal plus branch preservation is sufficient.

## Related

- `docs/adr/0005-switch-pi-sessions-between-registered-worktrees.md`
- `plugins/development-system/README.md`
- `plugins/development-system/skills/worktrees/SKILL.md`
- `plugins/development-system/skills/delivery/SKILL.md`
