---
title: Tell lock contention apart from lock-file errors
blocked_by: []
blocks: []
tags: [bug, worktrees, lefthook, diagnostics, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Give users different, actionable messages when the Lefthook installer is genuinely busy and when its lock file cannot be opened or created. Filesystem problems should retain their real cause instead of looking like another active install.

## Context / Why

Implementation notes:\n\nThe preliminary nonblocking flock currently maps every nonzero result to worktrees.hook_install_locked. That hides ordinary filesystem failures such as a read-only or missing state directory, exhausted space, and stale permissions. Preserve the contention diagnostic only when the lock was opened successfully and is actually held; lock-path/open failures need a separate stable diagnostic that retains the underlying filesystem cause and remediation.

## Acceptance criteria

- [ ] Actual nonblocking lock contention retains the existing locked diagnostic and retry guidance.
- [ ] Lock-file open or creation failures emit a distinct diagnostic that preserves the underlying filesystem error and recovery direction.
- [ ] Automated tests distinguish a successfully opened but held lock from unwritable, missing-parent, permission, and other lock-file open or creation failures.
- [ ] Each failure class has a stable machine-readable diagnostic and nonzero status while preserving safe, specific operator remediation.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Lower-frequency local-owner worktree/Lefthook operability or diagnostics. Existing guards provide the baseline protection, so this does not outrank current cross-project blockers, data-safety work, or the concrete dependency alert; no shadow queue.
