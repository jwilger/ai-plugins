---
name: tasks
description: Use whenever Tiber or origin/tiber is mentioned, including task workflows, EventCore publication conflicts, ambiguous pushes, tiber sync recovery, force-push safety, and cross-worktree task-board coordination.
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

When a task-board request coincides with a pushed-CI failure, hand the recovery
to Development Discipline's `ci-failure-follow-up` skill. Tiber remains a task
board; it neither grants nor releases the workflow hold.
