---
title: Fix hanging provider-backed eval runs
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Provider-backed marketplace evals must be reliable enough to use as completion evidence. Investigate recurring hangs, add bounded execution or timeout handling where appropriate, and make failures diagnosable instead of leaving agents stuck waiting indefinitely.

## Context / Why

## Acceptance criteria

- [ ] A reproduced or simulated hang is covered by tests or eval-runner fixtures that fail before the fix and pass after it.
- [ ] Provider-backed eval commands have bounded runtime behavior so a hung provider, harness, or child process cannot block indefinitely.

## Subtasks

## Notes / Log
