---
title: Distinguish Lefthook installer lock errors from active contention
blocked_by: []
blocks: []
tags: [worktrees, lefthook, diagnostics, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Give actionable diagnostics when opening the installer lock fails for reasons other than another active installer.

## Context / Why

Final review classified this operability observation as MINOR and non-blocking. The current preliminary flock call maps every nonzero result to worktrees.hook_install_locked, including routine filesystem failures such as a read-only state directory, exhausted space, or stale permissions.

## Acceptance criteria

- [ ] Actual nonblocking lock contention retains the existing locked diagnostic and retry guidance.
- [ ] Lock-file open or creation failures emit a distinct diagnostic that preserves the underlying filesystem error and recovery direction.

## Subtasks

## Notes / Log
