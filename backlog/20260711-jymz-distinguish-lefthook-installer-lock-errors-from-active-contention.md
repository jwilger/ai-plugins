---
title: Distinguish Lefthook installer lock errors from active contention
blocked_by: []
blocks: []
tags: [bug, worktrees, lefthook, diagnostics, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Give distinct, actionable diagnostics for active Lefthook installer contention versus failure to open or create the lock file.

## Context / Why

The preliminary nonblocking flock currently maps every nonzero result to worktrees.hook_install_locked. That hides ordinary filesystem failures such as a read-only or missing state directory, exhausted space, and stale permissions. Preserve the contention diagnostic only when the lock was opened successfully and is actually held; lock-path/open failures need a separate stable diagnostic that retains the underlying filesystem cause and remediation.

## Acceptance criteria

- [ ] Actual nonblocking lock contention retains the existing locked diagnostic and retry guidance.
- [ ] Lock-file open or creation failures emit a distinct diagnostic that preserves the underlying filesystem error and recovery direction.
- [ ] Automated tests distinguish a successfully opened but held lock from unwritable, missing-parent, permission, and other lock-file open or creation failures.

## Subtasks

## Notes / Log
