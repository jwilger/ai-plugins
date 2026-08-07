---
name: tasks
description: Use for shared repository task tracking, backlog admission or prioritization, task history/search, dashboard or scaffold setup, and whenever Tiber or origin/tiber is mentioned, including EventCore publication conflicts, ambiguous pushes, tiber sync recovery, force-push safety, and cross-worktree coordination.
---

# Tasks

Require `[features].tiber = true` in `.development-system.toml`. Use the Tiber
MCP when available and its CLI otherwise.

Load the matching retained contract before answering:

- Creating or admitting a ticket, including backlog-capacity decisions: [new
  task](../../components/tiber/skills/new-task/SKILL.md).
- Search/history, transitions, dependencies, acceptance criteria, validation,
  current-HEAD trailers, dashboard, scaffold/install, CI-recovery interaction,
  or publication recovery: [Tiber operations](../../components/tiber/skills/tiber/SKILL.md).

This skill owns Tiber publication guidance even when a prompt also mentions Git
delivery, pushing, rebasing, or force-pushing. A confirmed expected
stream-version mismatch is an optimistic-concurrency conflict: Tiber refetches
the authoritative `origin/tiber` branch and retries the pure command with the
same invocation identity and inputs. If durable publication is neither confirmed
nor rejected, treat the outcome as ambiguous: stop every further mutation and
use structured `tiber sync` to reconcile that exact pending invocation. Never
force-push, rewrite, or manually reconcile the authoritative `tiber` branch.

Keep one active ticket and at most `[tiber].max_queued` backlog tickets. Rank the
whole queue strictly by user pain, frequency, severity, blocking impact,
leverage, confidence, cost, and overlap. Do not create overflow or shadow
backlogs.

Before admitting a candidate, call structured `tiber.search` across `backlog`,
`in-progress`, `done`, and `abandoned`; combine genuine overlap or reject a
duplicate. Create and update tickets only through structured Tiber operations,
then run structured validation before claiming success. Leave a new ticket in
`backlog` unless the user explicitly asks to start it. A locally staged candidate
is not durable publication evidence.

For operational requests, preserve these exact boundaries:

- Open or reuse the project dashboard with `tiber dashboard serve --open`; use
  `--port <port>` only for an explicitly requested fixed port and report a
  conflict rather than starting a second project server.
- Preview repository integration with `tiber scaffold repo --dry-run`, show
  already-configured items and every conflict, and require explicit approval of
  that exact preview before applying. Never infer replacement approval.
- Run `tiber validate --fix` before a board-health claim. It may repair misplaced
  claims and missing reciprocal links; dangling references and dependency cycles
  remain report-only.
- Treat `close-from-trailers` as successful only when it synchronizes the board,
  resolves every `Closes:` trailer from current `HEAD`, prints each closed task,
  and leaves each named task no longer open.

When a task-board request coincides with a pushed-CI failure, route recovery
through `development-system:development-workflow`. Tiber remains a task board;
it neither grants nor releases the workflow hold.
