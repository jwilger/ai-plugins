---
title: Clarify Lefthook crash recovery when descendants retain the installer lock
blocked_by: []
blocks: []
tags: [bug, worktrees, lefthook, documentation, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Correct Lefthook installer recovery guidance to explain that an inherited flock remains held until the last surviving lock-inheriting descendant exits.

## Context / Why

AGENTS currently says flock releases after a crash, but the intentional no-fork behavior means leader death is insufficient when a descendant retains the descriptor. Update every canonical recovery surface to distinguish those cases and direct the operator to wait for or terminate the surviving process group before retrying. This is documentation of real contention, distinct from the false-contention diagnostic bug in 20260711-jymz.

## Acceptance criteria

- [ ] Recovery guidance distinguishes leader exit from the last lock-inheriting descendant exiting and tells the user to wait for or terminate the surviving process group before retrying.

## Subtasks

## Notes / Log
