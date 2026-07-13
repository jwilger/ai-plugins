---
title: Prove exact GPT-5.6 benchmark workspace ownership marker contents
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, tests, filesystem, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a negative regression proving that only the exact benchmark workspace marker contents authorize recursive recreation.

## Context / Why

Current tests cover an absent marker and the exact valid marker, but not a near-match or malformed marker. A regression accepting any marker file would pass. This MINOR review finding was deferred from 20260709-spx8.

## Acceptance criteria

- [ ] A nonempty workspace with a near-match marker is refused and all existing content is preserved.

## Subtasks

## Notes / Log
