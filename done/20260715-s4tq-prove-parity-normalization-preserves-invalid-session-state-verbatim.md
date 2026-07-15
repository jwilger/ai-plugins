---
title: Prove parity normalization preserves invalid session state verbatim
blocked_by: []
blocks: [20260715-w7ga-scope-parity-clock-normalization-by-review-session]
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen release-parity coverage so malformed review state is proven byte-for-byte/structurally unchanged rather than merely different across source and distribution transcripts.

## Context / Why

Formal review finding tests.missing-session-preservation-not-proven: the missing-session regression can pass after partial normalization because both contract ID and timestamp differ. Add direct preservation assertions and exercise a present-but-invalid stable session_id boundary.

## Acceptance criteria

- [x] The missing-session fixture asserts each normalized record is unchanged from its own input, so partial normalization is detected.
- [x] A present-but-invalid session_id fixture proves noncanonical review state is preserved without partial normalization.
- [x] The strengthened regression fails against a partial-normalization mutation and all focused/full repository gates pass.

## Subtasks

## Notes / Log

- 2026-07-15: Completed in 9e11f9adaf957dbcec62b2386d9becff979268c1. The strengthened assertion failed under a temporary partial-normalization mutation, then all 16 focused release tests, source/distribution parity, and full `just ci` passed (242 development-discipline tests, 38 caught/6 unviable mutants, 354 Bats). Formal low-risk tests-verification review completed clean with no out-of-scope findings.
