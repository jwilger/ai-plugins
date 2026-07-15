---
title: Label parity normalization failures by side and record
blocked_by: []
blocks: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make parity normalization failures identify source versus distribution and the failing JSONL record before temporary artifacts are cleaned.

## Context / Why

Formal review finding parity-normalizer-loses-failure-context: set -e exits on unlabeled normalizer errors before the parity marker/diff, while the EXIT trap deletes raw captures.

## Acceptance criteria

- [x] Blank and invalid JSONL failures identify the input and one-based record number.
- [x] The parity shell emits a side-specific source or distribution normalization failure marker before cleanup.
- [x] Focused failure-path coverage and full repository gates pass.

## Subtasks

## Notes / Log

- 2026-07-15: Implemented and committed as cfa46bd2a45491138f78740fa2e6f81705e19990. TDD evidence: focused tests first failed because blank/invalid JSONL errors lacked input and record context and the wrapper lacked side markers. The final tests cover blank and invalid record 2, both source and distribution failures, opposite-side exclusion, and marker ordering before the EXIT cleanup sentinel. All 19 focused release Bats tests, the real source/distribution parity gate, and full `nix develop -c just ci` passed (242 development-discipline tests; 38 caught/6 unviable mutants; 357 Bats tests). Formal final review parity-diagnostics-final-cfa46bd-v1 completed clean with no findings or out-of-scope observations.
