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
