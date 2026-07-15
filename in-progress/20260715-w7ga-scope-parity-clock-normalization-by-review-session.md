---
title: Scope parity clock normalization by review session
blocked_by: [20260715-s4tq-prove-parity-normalization-preserves-invalid-session-state-verbatim]
blocks: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
tags: [development-discipline, release-parity, tests, bug]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Make review-budget timestamp canonicalization independent across stable review sessions while preserving timestamp relationships within each session.

## Context / Why

Formal combined review of 20260715-yvha found that the transcript-global timestamp map preserves incidental equality across independent sessions. If two sessions start in the same epoch second in one binary run but straddle a boundary in the other, identical behavior normalizes differently. Key timestamp ordinals by canonical state.session_id, fail closed when the otherwise canonical state lacks a valid session ID, and add focused cross-session and within-session regressions. This task blocks 20260715-yvha.

## Acceptance criteria

- [x] Equivalent transcripts normalize equally when independent review sessions share one source timestamp but straddle timestamps in the distribution run.
- [ ] Within one stable review session, retained and reset review-budget timestamps remain distinguishable.
- [ ] The normalizer only applies session-scoped clock canonicalization to canonical review state with a valid stable session_id, and focused/full CI gates pass.

## Subtasks

## Notes / Log
