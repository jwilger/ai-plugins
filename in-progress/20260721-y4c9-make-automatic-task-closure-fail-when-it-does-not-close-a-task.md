---
title: Make automatic task closure fail when it does not close a task
blocked_by: []
blocks: []
tags: [tiber, bug, automation, task-state, high-priority]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

When a main-branch commit says that a Tiber task is closed, automation must either move that task to Done or fail with a useful explanation. A green workflow must never leave completed work incorrectly shown as active.

## Context / Why

GitHub issue 56 reports two successful workflows where the close-from-trailers command found a task-closing commit message but left the named tasks in progress. This makes the dashboard inaccurate, breaks one-ticket-at-a-time coordination, and causes later task updates to collide with stale state. The repair should report which tasks were closed and return a failure for missing tasks, invalid task data, synchronization conflicts, push failures, or any other condition that prevents a requested closure.

## Acceptance criteria

- [x] A valid task-closing line in a newly pushed main-branch commit moves every named in-progress task to Done and publishes the updated board.
- [x] The command prints the task identifiers that it successfully closed.
- [x] The command exits with a failure and a specific explanation when any requested task cannot be closed.
- [x] A successful workflow guarantees that every task named for closure is no longer in progress.
- [ ] Regression tests cover both reproductions documented in GitHub issue 56 and the later synchronization-conflict symptom.

## Subtasks

## Notes / Log

- 2026-07-21: Created while triaging GitHub issue #56: https://github.com/jwilger/ai-plugins/issues/56
