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

## Acceptance criteria

## Subtasks

## Notes / Log
