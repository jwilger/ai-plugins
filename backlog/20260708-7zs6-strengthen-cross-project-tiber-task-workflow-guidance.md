---
title: Strengthen cross-project Tiber task workflow guidance
blocked_by: []
blocks: []
tags: [tiber, workflow, worktrees, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Complete the remaining cross-project Tiber workflow guidance for concurrent active-task isolation and PR/MR lifecycle tracking while preserving already-delivered backlog capture and transition behavior.

## Context / Why

Backlog-only capture, explicit transition before active work, and PR/MR status fields already exist through completed Tiber work. The remaining delta is to make concurrent in-progress isolation unambiguous, ensure claims/branches/worktrees stay one-per-active-task, and add missing behavior coverage for PR URL and status changes through review and merge. Do not reimplement the existing new-task or transition guidance.

## Acceptance criteria

- [ ] Tiber guidance makes clear that backlog capture leaves tasks in backlog unless the user explicitly asks to start work.
- [ ] Guidance requires active work to move tasks to in-progress and requires multiple in-progress tasks to be isolated by separate branches or worktrees.
- [ ] The change includes eval cases for backlog-only capture, explicit start-work transition, concurrent in-progress isolation, and PR/MR status updates.

## Subtasks

## Notes / Log
