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

## Subtasks

## Notes / Log
