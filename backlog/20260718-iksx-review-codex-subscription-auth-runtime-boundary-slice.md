---
title: Review Codex subscription-auth runtime boundary slice
blocked_by: []
blocks: []
tags: [evals, codex, auth, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Independently review and ship the Codex subscription-auth runtime isolation boundary required by 20260715-n6bs.

## Context / Why

Split from 20260715-n6bs after formal final review returned scope_split_hold for an unusually broad new subsystem. This slice covers isolated reuse of existing Codex CLI ChatGPT-subscription authentication, candidate sandbox boundaries, lifecycle limits, and fail-closed cleanup.

## Acceptance criteria

- [ ] The runtime protects operator source authentication while providing disposable run-scoped authentication that can refresh normally.
- [ ] Candidate execution cannot access authentication sources, host paths, sibling candidates, or the network.
- [ ] Resource limits and cleanup are finite and fail closed on setup or boundary failure.

## Subtasks

## Notes / Log
