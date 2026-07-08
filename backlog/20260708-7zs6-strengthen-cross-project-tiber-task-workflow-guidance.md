---
title: Strengthen cross-project Tiber task workflow guidance
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen the Tiber plugin guidance so agents in any repository handle task state deliberately: capture backlog tasks without starting them, move active work to in-progress, isolate concurrent work by branch/worktree, and keep PR/MR status current.

## Context / Why

## Acceptance criteria

- [ ] Tiber guidance makes clear that backlog capture leaves tasks in backlog unless the user explicitly asks to start work.
- [ ] Guidance requires active work to move tasks to in-progress and requires multiple in-progress tasks to be isolated by separate branches or worktrees.
- [ ] The change includes eval cases for backlog-only capture, explicit start-work transition, concurrent in-progress isolation, and PR/MR status updates.

## Subtasks

## Notes / Log
