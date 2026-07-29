# ADR-0005: Switch Pi sessions between registered worktrees

## Status

Superseded by ADR-0006

## Date

2026-07-29

## Context

ADR-0004 repaired primary-checkout bootstrap by adding semantic worktree
inventory and creation tools. It correctly rejected the fiction that a shell
`cd` or `git -C` changes the current Pi runtime, but required the user to start a
new Pi process in the returned path. In autonomous goals this process boundary
looked like an external blocker even after the linked worktree existed.

Marketplace research found three patterns: return a relaunch command, virtually
rewrite a subset of tool paths and bash commands, or replace the Pi session with
one whose cwd is the target worktree. Pi 0.82 exposes the last operation as the
public `ExtensionCommandContext.switchSession()` API.

## Decision

Add `/development-system-worktree-switch` as a local-TUI command for switching
the active conversation to an already registered worktree. Wait until Pi is
idle, require explicit confirmation, revalidate the selected path, branch, and
HEAD, serialize the active session-tree branch into a private mode-0600 Pi
session whose canonical cwd is the target, and invoke the public session
replacement API. Verify the replacement context cwd before reporting success
and never access the stale source context afterward.

Use `@narumitw/pi-worktree` as the attributed behavioral and architectural
reference. Implement a narrow repository-native adapter without installing,
vendoring, or depending on the package.

Do not implement virtual tool routing. It would leave the actual session cwd,
custom tools, context/resource loading, and project trust bound to the source
checkout. Do not expose session switching as a model tool: Pi intentionally
makes `switchSession()` available only to a user-initiated command context.
Model-callable list/create tools return the exact switch command instead.

Retain the exact `cd ... && exec pi` command as the fallback for JSON, print,
RPC, or other headless callers. Preserve prepared private sessions and Git
worktrees after cancellation or failure; perform no speculative cleanup.

## Consequences

### Positive

- A local Pi TUI can continue the same conversation in another worktree without
  restarting the Pi process.
- Pi itself rebuilds every cwd-bound runtime surface rather than the extension
  spoofing only built-in tools.
- Active conversation branches and development-system custom entries, including
  bounded goal state, move through the documented session representation.
- The switch adds no process spawn, shell interpolation, auth copy, dependency,
  worktree deletion, or branch mutation authority.

### Negative

- The user must initiate and confirm the command; an autonomous model cannot
  silently switch workspaces.
- The active conversation is copied into another private session file, and a
  failed switch can leave that recovery file behind.
- Headless modes still require a new process in the target worktree.
- Session serialization and replacement are coupled to the supported Pi 0.82
  API and must be revalidated when the compatibility range changes.

## Alternatives Considered

### Rewrite built-in tool paths and prefix bash with `cd`

Rejected because it does not rebind custom tools, extension status, resources,
context files, trust, or the actual Pi session cwd. Absolute paths and future
tools would continually expand the mediation surface.

### Spawn a replacement Pi process or terminal pane

Rejected as the default because it inherits environment and terminal lifecycle
complexity, requires external terminal orchestration, and is unnecessary when
Pi can replace the session in process.

### Let a model tool call `switchSession()`

Rejected because Pi exposes workspace replacement only on command contexts.
Casting or reaching into runner internals would be unsupported and would remove
the explicit transcript-transfer approval.

### Continue requiring relaunch for every mode

Rejected for the local TUI because it turns a completed semantic bootstrap into
a recurring autonomous-goal handoff blocker.

## Related

- `docs/adr/0004-bootstrap-linked-worktrees-with-semantic-pi-tools.md`
- `docs/research/pi-worktree-session-switching.md`
- `plugins/development-system/README.md`
- `plugins/development-system/skills/worktrees/SKILL.md`
