---
title: Clarify Lefthook crash recovery when descendants retain the installer lock
blocked_by: []
blocks: []
tags: [worktrees, lefthook, documentation, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make installer recovery guidance explicit that an inherited flock remains held until the last surviving process in the original installer group exits.

## Context / Why

Final review classified this documentation mismatch as MINOR and non-blocking. AGENTS currently says flock releases after a crash, but the intentional --no-fork behavior means a surviving descendant can retain the descriptor and keep retries locked until that process group exits.

## Acceptance criteria

## Subtasks

## Notes / Log
