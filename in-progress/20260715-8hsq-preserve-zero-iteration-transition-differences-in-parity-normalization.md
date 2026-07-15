---
title: Preserve zero-iteration transition differences in parity normalization
blocked_by: []
blocks: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Treat iteration zero as malformed parity state so its transition ID remains observable.

## Context / Why

Formal review finding parity-normalizer-accepts-zero-iteration: runtime verified_clean_iterations start at 1, but the normalizer currently accepts 0 and can mask differing malformed transition IDs.

## Acceptance criteria

- [ ] A focused fixture with iteration 0 and differing transition IDs remains different after normalization.
- [ ] Only positive safe-integer verified iteration values have transition IDs normalized.
- [ ] The regression demonstrates the pre-fix masking behavior, then focused parity and full repository gates pass.

## Subtasks

## Notes / Log
