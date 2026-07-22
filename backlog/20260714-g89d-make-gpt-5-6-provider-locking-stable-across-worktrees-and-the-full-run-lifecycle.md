---
title: Prevent overlapping GPT-5.6 benchmark runs across worktrees
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, locking, concurrency, worktrees, process-lifecycle, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Use one reliable lock for the entire GPT-5.6 benchmark lifecycle so separate worktrees cannot run the protected provider at the same time. The lock must remain stable despite symbolic links, disposable caches, and post-run checking.

## Context / Why

Implementation notes:\n\nSplit from 20260713-uf3e. The focused and canonical runners can disagree on logical versus canonical lock paths, the lock currently lives under disposable .dependencies state, and coverage does not prove retention through post-run checking, dry-run non-creation, or resistance to unlink/recreation splitting. This task owns only shared provider-lock identity and lifecycle.

## Acceptance criteria

- [ ] A successful live run proves the shared provider lock and its inherited identity remain held during provider execution and post-run artifact checking, then are released after the complete lifecycle.
- [ ] Dry-run neither acquires nor creates the provider lock.
- [ ] Focused and canonical runners derive the same canonical lock identity when invoked through a symlinked checkout or runner path.
- [ ] The cross-worktree lock lives outside disposable .dependencies caches, and inability to resolve the Git-common location fails closed before provider launch.
- [ ] Regression coverage proves unlinking or recreating disposable paths cannot produce a second concurrently acquirable lock.

## Subtasks

## Notes / Log

- 2026-07-14: Split from 20260713-uf3e. This ticket exclusively owns shared provider-lock identity, storage, acquisition, inheritance, and release.
- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
