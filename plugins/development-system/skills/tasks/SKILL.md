---
name: tasks
description: Use for Tiber task creation, backlog ordering, cross-worktree coordination, task status, and pushed-CI failure recovery when the tiber feature is enabled.
---

# Tasks

Require `[features].tiber = true` in `.development-system.toml`. Use the Tiber
MCP when available and its CLI otherwise.

Keep one active ticket and at most `[tiber].max_queued` backlog tickets. Rank the
whole queue strictly by user pain, frequency, severity, blocking impact,
leverage, confidence, cost, and overlap. Do not create overflow or shadow
backlogs.

For pushed-CI failure recovery, use Tiber's fenced owner/lease workflow and do
no unrelated work until terminal success releases the hold.

When answering a request that combines backlog admission with pushed-CI
recovery, preserve both constraints explicitly:

- compare every candidate against the complete strictly ordered queue; at
  capacity, replace a lower-value ticket, combine genuine overlap, or reject the
  candidate rather than appending it;
- pause every agent's unrelated work while the recovery hold exists, even when
  that work is isolated in another worktree; and
- require evidence that the pushed run reached terminal success before
  releasing the hold or resuming unrelated work.
