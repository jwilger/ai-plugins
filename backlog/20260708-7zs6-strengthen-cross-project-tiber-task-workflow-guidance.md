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

- [ ] Tiber guidance makes clear that backlog capture leaves tasks in backlog unless the user explicitly asks to start work.
- [ ] Guidance requires active work to move tasks to in-progress and requires multiple in-progress tasks to be isolated by separate branches or worktrees.
- [ ] The change includes eval cases for backlog-only capture, explicit start-work transition, concurrent in-progress isolation, and PR/MR status updates.

## Subtasks

## Notes / Log
