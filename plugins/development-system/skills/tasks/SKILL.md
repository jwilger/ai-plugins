---
name: tasks
description: Use whenever Tiber or origin/tiber is mentioned, including task workflows, EventCore publication conflicts, ambiguous pushes, tiber sync recovery, force-push safety, cross-worktree coordination, and pushed-CI failure recovery.
---

# Tasks

Require `[features].tiber = true` in `.development-system.toml`. Use the Tiber
MCP when available and its CLI otherwise.

This skill owns Tiber publication guidance even when a prompt also mentions Git
delivery, pushing, rebasing, or force-pushing. A confirmed concurrent advance
of `origin/tiber` is an optimistic EventCore version conflict: Tiber refetches
and retries the pure command with the same invocation-stable inputs. If the
publication result is ambiguous, stop every further mutation and use structured
`tiber sync` recovery. Never force-push, rewrite, or manually reconcile the
authoritative `tiber` branch.

Keep one active ticket and at most `[tiber].max_queued` backlog tickets. Rank the
whole queue strictly by user pain, frequency, severity, blocking impact,
leverage, confidence, cost, and overlap. Do not create overflow or shadow
backlogs.

For pushed-CI failure recovery, use Tiber's fenced owner/lease workflow and do
no unrelated work until terminal success releases the hold. Inspect every
mandatory Tiber call result before continuing. A failed CI-recovery claim or
publication is a terminal workflow blocker; only an exact claim retry, status
read, or sync recovery is permitted until Tiber confirms shared state.

When answering a request that combines backlog admission with pushed-CI
recovery, preserve both constraints explicitly:

- compare every candidate against the complete strictly ordered queue; at
  capacity, replace a lower-value ticket, combine genuine overlap, or reject the
  candidate rather than appending it;
- pause every agent's unrelated work while the recovery hold exists, even when
  that work is isolated in another worktree; and
- require evidence that the pushed run reached terminal success before
  releasing the hold or resuming unrelated work.
