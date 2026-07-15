---
title: Preserve review-budget timestamp relationships in parity normalization
blocked_by: []
blocks: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
tags: [development-discipline, release-parity, tests, bug]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Preserve the equality pattern of runtime review-budget start timestamps while normalizing cross-run clock drift in development-discipline release parity.

## Context / Why

Formal final review of ticket 20260715-yvha found that replacing every started_at_epoch_seconds value with zero masks a packaged binary that resets the review budget between transitions. Canonicalize distinct timestamps by first occurrence so cross-run absolute values disappear while within-transcript equality relationships remain observable. Add a focused two-record regression before changing the normalizer. This task blocks 20260715-yvha.

## Acceptance criteria

- [x] The parity normalizer removes absolute clock drift while preserving the equality pattern of started_at_epoch_seconds values within a transcript.
- [x] Focused coverage fails when source retains a review-budget start time but the distribution resets it between records.
- [x] The focused release suite, release-from-source parity gate, and full repository CI pass at the fixed commit.

## Subtasks

## Notes / Log

- 2026-07-15: Implemented at da1c9863f5754cff17ccd8c03b9c36013d5c0f65. TDD proved the original false negative: retained [100,100] and reset [101,102] both collapsed to [0,0]. First-seen ordinal normalization now preserves timestamp equality relationships. Evidence: focused regression and all 14 release tests green; source/dist parity green; `nix develop -c just ci` green with 242 development-discipline tests, mutation testing 38 caught/6 unviable, and all 352 Bats. Formal final review session clock-rel-final-da1c986-v1 completed clean with no out-of-scope findings.
