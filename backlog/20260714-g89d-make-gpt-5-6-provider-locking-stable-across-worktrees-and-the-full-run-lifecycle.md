---
title: Make GPT-5.6 provider locking stable across worktrees and the full run lifecycle
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, locking, concurrency, worktrees, process-lifecycle, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Hold one canonical provider lock from execution through artifact checking, keep its identity stable across symlinked checkouts and disposable-cache changes, and fail closed when shared lock identity cannot be resolved.

## Context / Why

Split from 20260713-uf3e. The focused and canonical runners can disagree on logical versus canonical lock paths, the lock currently lives under disposable .dependencies state, and coverage does not prove retention through post-run checking, dry-run non-creation, or resistance to unlink/recreation splitting. This task owns only shared provider-lock identity and lifecycle.

## Acceptance criteria

- [ ] A successful live run proves the shared provider lock and its inherited identity remain held during provider execution and post-run artifact checking, then are released after the complete lifecycle.
- [ ] Dry-run neither acquires nor creates the provider lock.
- [ ] Focused and canonical runners derive the same canonical lock identity when invoked through a symlinked checkout or runner path.
- [ ] The cross-worktree lock lives outside disposable .dependencies caches, and inability to resolve the Git-common location fails closed before provider launch.

## Subtasks

## Notes / Log
