# ADR-0003: Making a running side-quest's progress inspectable

## Status

Accepted

## Date

2026-07-01

## Context

A side-quest's goal session runs as a detached OS process (`sidequest
run-quest`, spawned by the `launch` MCP tool and outliving it), driving a
headless harness invocation inside an isolated worktree. A user launching a
side-quest has no way today to see what it is actually doing while it runs, or
after it finishes short of manually inspecting the worktree's git state.
`session::run` previously used `Command::output()`, which buffers the child's
stdout/stderr in memory and discards it on success -- so even a manual
post-mortem of a successful run had nothing to inspect.

We researched what Claude Code's MCP integration actually supports for
surfacing a background process's progress:

- **The native Claude Code "agents" list is populated exclusively by
  subagents Claude Code itself spawns via its Task tool.** An MCP-spawned OS
  process is not a Claude Code Task; there is no MCP capability, resource, or
  notification that injects an external process into that list. This is a
  structural limitation, not a gap we can close from the server side.
- **`claude/channel`** (an experimental MCP capability) injects server-pushed
  content into Claude's own context window for the _model_ to react to on its
  next turn -- it is not a user-visible progress UI, is gated behind
  `--dangerously-load-development-channels` for non-allowlisted servers, and
  has documented reliability issues (notifications silently dropped). Wrong
  abstraction for "let the user watch progress."
- **MCP resources** (`resources/list`/`read`) are supported by both `rmcp` and
  Claude Code, but Claude Code does not implement `resources/subscribe` /
  `notifications/resources/updated` -- there is no live-update path; a
  resource would still need to be manually re-read, exactly like a tool call.
- **A plain polling MCP tool that reads a log file** is the most direct,
  reliable mechanism available today, and needs no unstable/experimental MCP
  capability.

## Decision

- `session::run` redirects the harness child's stdout and stderr directly to a
  per-side-quest log file (two independently-opened handles to the same path,
  both in append mode, so the OS serializes writes rather than an in-process
  buffer-and-copy loop) instead of capturing them in memory. The log fills in
  live, on disk, as the session runs.
- The log path is `.git/sidequest/logs/<branch-with-slashes-flattened>.log`
  (`logs::path`), alongside the existing registry, for the same reasons (per
  project, shared across worktrees, never a tracked change).
- A new `logs` MCP tool reads a side-quest's log back, tail-first (defaulting
  to the last 200 lines) given its branch. This works identically whether the
  side-quest is still running or already finished.
- We explicitly do **not** attempt `claude/channel`, MCP resource
  subscriptions, or any mechanism promising the side-quest will appear in
  Claude Code's native agents list -- those either don't exist, are unreliable
  research-preview features, or are structurally impossible for an externally
  spawned process. The user can poll `list` (for state) and `logs` (for
  output) manually, or via the existing `/loop` mechanism for periodic checks.

## Consequences

### Positive

- "Is there a way for me to view its progress?" now has a real answer that
  works today, for any side-quest, without opt-in flags or unstable features.
- The log capture is a byproduct of how the session already runs -- no new
  process supervision or IPC mechanism was introduced.
- Reading logs works the same whether the side-quest is `Running`, `Failed`,
  `Done`, or `Delivered` -- one mechanism for both live and post-mortem
  inspection.

### Negative

- Not truly "live" from the user's perspective -- inspecting progress means
  calling a tool again, not watching a stream update in place. This is the
  ceiling of what Claude Code's MCP client supports today, not a limitation of
  this implementation.
- The two file handles into the same append-mode log file are not
  synchronized beyond what `O_APPEND` guarantees; under heavy concurrent
  stdout/stderr output, individual lines could in principle interleave.
  Acceptable for a human/AI skimming progress; not a structured log format.
- No session/thread id is captured for resuming the underlying harness
  session (`claude`/`codex` both support this via their JSON output modes).
  Left for a future slice; the log itself contains that id in its first line
  today for whichever harness was used.

## Alternatives Considered

### `claude/channel` capability

Rejected: wrong abstraction (feeds the model's context, not a user-visible
UI), experimental/gated, and documented as unreliable (dropped notifications).

### MCP resources with subscription

Rejected: Claude Code does not implement `resources/subscribe` /
`notifications/resources/updated` today, so a resource would need manual
re-reading anyway -- no better than a tool call, with more moving parts.

### Attempt to register the worker as a Claude Code Task/subagent

Rejected: structurally impossible. The native agents list is populated only by
processes Claude Code itself spawns via its Task tool; an MCP tool call cannot
inject an external OS process into it.

## Revisit when

Claude Code's MCP client adds resource-subscription support, or a
non-experimental, reliable server-push mechanism ships -- either would let
`logs` become a genuinely live stream instead of a poll.

## Related

- ADR-0001 (overall architecture)
- ADR-0002 (harness invocation)
