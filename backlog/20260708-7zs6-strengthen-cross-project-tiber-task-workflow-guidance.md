---
title: Enforce concurrent Tiber task isolation and PR/MR lifecycle coverage
blocked_by: []
blocks: []
tags: [tiber, workflow, worktrees, pr-mr, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Complete the remaining Tiber workflow guidance and behavior coverage for one-branch-or-worktree-per-active-task isolation and PR/MR status progression.

## Context / Why

Backlog-only capture, explicit transition before active work, and basic PR/MR URL and status guidance are already delivered. Preserve those behaviors through regression coverage but do not reimplement them. The remaining gap is unambiguous isolation for multiple in-progress tasks and end-to-end PR/MR lifecycle fixtures.

## Acceptance criteria

- [ ] Guidance requires each concurrently active in-progress task to use its own claim, branch, and linked worktree and forbids treating a backlog claim as an informal reservation.
- [ ] PR/MR lifecycle coverage records the URL and stable status changes through draft or open, checks, review, approval, merge, close, or blocked outcomes.
- [ ] Behavior fixtures cover concurrent in-progress isolation and PR/MR lifecycle progression while retaining regression cases for already-delivered backlog-only capture and explicit start-work transition.

## Subtasks

## Notes / Log

- 2026-07-14: Backlog grooming 2026-07-14: Removed already-delivered implementation scope for backlog capture, transition, and basic PR fields. Those remain regression expectations; this ticket now owns concurrent isolation and lifecycle coverage.
