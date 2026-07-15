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

- [x] A focused fixture with iteration 0 and differing transition IDs remains different after normalization.
- [x] Only positive safe-integer verified iteration values have transition IDs normalized.
- [x] The regression demonstrates the pre-fix masking behavior, then focused parity and full repository gates pass.

## Subtasks

## Notes / Log

- 2026-07-15: Implemented and committed as 1ffdf41f9711a2366a6c9ee99f4e2a8d7fb96777. TDD evidence: the isolated iteration-0 fixture failed before the guard change because distinct malformed transition IDs normalized equal; after requiring a positive safe integer, all 17 focused release/parity tests and the source/distribution parity gate passed. Full `nix develop -c just ci` passed (242 development-discipline tests; mutation score 38 caught, 6 unviable; 355 Bats tests). Formal final review session zero-iteration-final-1ffdf41-v1 completed clean with one correctness lens, no findings, and no out-of-scope observations.
