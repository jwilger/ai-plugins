---
title: Explain when a crashed hook installer still holds its lock
blocked_by: []
blocks: []
tags: [bug, worktrees, lefthook, documentation, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Correct the recovery guidance for a Lefthook installer crash. If a surviving child process still holds the shared lock, users must wait for or stop that process group before retrying; the original installer process ending is not always enough.

## Context / Why

Implementation notes:\n\nAGENTS currently says flock releases after a crash, but the intentional no-fork behavior means leader death is insufficient when a descendant retains the descriptor. Update every canonical recovery surface to distinguish those cases and direct the operator to wait for or terminate the surviving process group before retrying. This is documentation of real contention, distinct from the false-contention diagnostic bug in 20260711-jymz.

## Acceptance criteria

- [ ] Recovery guidance distinguishes leader exit from the last lock-inheriting descendant exiting and tells the user to wait for or terminate the surviving process group before retrying.
- [ ] AGENTS and any installer recovery documentation use the same corrected descendant-held-lock explanation and remediation.
- [ ] The existing regression coverage proving that a surviving child retains the lock remains passing and is referenced by the documentation change.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Lower-frequency local-owner worktree/Lefthook operability or diagnostics. Existing guards provide the baseline protection, so this does not outrank current cross-project blockers, data-safety work, or the concrete dependency alert; no shadow queue.
