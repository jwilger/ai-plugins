---
title: Retire superseded Lefthook Nix GC roots after successful reinstall
blocked_by: []
blocks: []
tags: [worktrees, lefthook, nix, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Avoid unbounded repository-local Nix store retention when the pinned Lefthook derivation changes.

## Context / Why

Final review classified this architecture/operability/production-risk observation as MINOR and non-blocking. The installer names indirect GC roots by full store-path basename and currently leaves older installer-owned roots after a later successful reinstall. Cleanup must happen only after the new config and all launchers are installed so interrupted mixed states remain runnable.

## Acceptance criteria

- [ ] After a successful reinstall, only the active installer-owned Lefthook GC root remains.
- [ ] Interrupted or failed reinstalls retain every GC root still needed by an active old-or-new launcher state.

## Subtasks

## Notes / Log
