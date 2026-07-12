---
title: Retire superseded Lefthook Nix GC roots after successful reinstall
blocked_by: []
blocks: []
tags: [bug, worktrees, lefthook, nix, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Retire obsolete installer-owned Lefthook Nix GC roots after a fully successful reinstall without deleting any root still referenced by a surviving old-or-new launcher state.

## Context / Why

The installer names indirect roots by Lefthook store-path basename and currently retains every older root. Cleanup must run only after the new config and all launchers are atomically installed. Ownership is limited to symlinks in the repository-managed roots directory that match the installer naming contract; foreign files, non-symlinks, and roots referenced by any viable interrupted state must be preserved.

## Acceptance criteria

- [ ] After a successful reinstall, only the active installer-owned Lefthook GC root remains.
- [ ] Interrupted or failed reinstalls retain every GC root still needed by an active old-or-new launcher state.
- [ ] Cleanup removes only obsolete installer-owned root symlinks and preserves foreign entries, non-symlinks, the active root, and every root referenced by a surviving launcher.
- [ ] Tests cover an old-to-new successful reinstall and injected failures before and during launcher replacement, proving interrupted states remain runnable.

## Subtasks

## Notes / Log
