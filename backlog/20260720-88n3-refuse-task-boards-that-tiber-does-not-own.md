---
title: Refuse task boards that Tiber does not own
blocked_by: []
blocks: []
tags: [tiber, task-board, data-safety]
pr_mr_url: 
pr_mr_status: 
---

## Summary

A repository may already have a task board created by another tool. Tiber currently tries to read and merge that incompatible data, which produces misleading conflicts and validation errors. Make Tiber identify boards it created and stop safely when it encounters a different board, so existing planning data is not changed or misrepresented.

## Context / Why

Covers GitHub issue #54. Implementation notes: Add durable commit metadata that identifies Tiber and its version on every commit Tiber writes to the Git tasks branch. Before any read or write, inspect the current tasks-branch head commit. Missing ownership metadata must produce an actionable error and no mutation. An explicit migration may establish ownership by creating a Tiber-stamped head commit; automatic conversion of legacy content is outside this ticket.

## Acceptance criteria

- [ ] Every commit Tiber writes to the task-storage branch includes durable metadata identifying Tiber and the writing version.

## Subtasks

## Notes / Log
