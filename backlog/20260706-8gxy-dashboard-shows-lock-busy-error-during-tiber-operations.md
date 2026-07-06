---
title: Dashboard shows lock-busy error during Tiber operations
blocked_by: []
blocks: []
tags: [tiber, dashboard, bug, locking]
---

## Summary

When any Tiber operation runs, the dashboard refreshes to a page containing only the lock error instead of keeping the last good board state or showing a non-destructive transient status.

## Context / Why

Observed while using the dashboard on this repository: each Tiber CLI operation causes the live dashboard refresh path to render only:

`tiber.parse_error tiber_lock_busy path=/home/jwilger/projects/ai-plugins/.git/tiber/tiber.lock`

The dashboard should tolerate the short-lived writer lock used by task operations. While filing this ticket, `tiber update` on the newly created task also failed with `sync_conflict` against the same task file after applying the intended local edit, so structured edits may be running post-mutation sync in a way that compares intended local changes against the current remote tree as conflicts.

## Acceptance criteria

- [ ] Dashboard does not replace the board with a full-page `tiber_lock_busy` error during normal Tiber writes.
- [ ] Dashboard either keeps rendering the last good snapshot, shows a scoped transient syncing/locked state, or retries after the lock clears.
- [ ] Read-only dashboard refreshes never contend with normal Tiber write locks in a way that creates user-visible fatal errors.
- [ ] Structured edit commands such as `tiber update` can edit an existing synced task without self-conflicting against the remote copy.
- [ ] Regression coverage proves dashboard refresh behavior during a concurrent Tiber operation.

## Subtasks

## Notes / Log
