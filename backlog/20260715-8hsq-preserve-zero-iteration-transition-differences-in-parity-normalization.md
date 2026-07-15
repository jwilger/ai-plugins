---
title: Preserve zero-iteration transition differences in parity normalization
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Treat iteration zero as malformed parity state so its transition ID remains observable.

## Context / Why

Formal review finding parity-normalizer-accepts-zero-iteration: runtime verified_clean_iterations start at 1, but the normalizer currently accepts 0 and can mask differing malformed transition IDs.

## Acceptance criteria

- [ ] A focused fixture with iteration 0 and differing transition IDs remains different after normalization.

## Subtasks

## Notes / Log
