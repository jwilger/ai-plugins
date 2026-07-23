---
title: Stop obsolete continuous integration runs safely
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

New pushes to the main branch can leave older validation runs consuming roughly twenty to twenty-five minutes of duplicate work. Cancel runs made obsolete by a newer revision while preserving the authoritative result for the latest pushed commit.

## Context / Why

Reducing redundant validation shortens feedback time and avoids wasting hosted runner capacity. Cancellation must never hide a failure for the latest revision, disrupt pull-request or merge-queue guarantees, or weaken the terminal-green delivery rule.

## Acceptance criteria

- [ ] A newer push safely cancels obsolete runs for the same delivery stream while the latest revision continues to a terminal result.

## Subtasks

## Notes / Log
