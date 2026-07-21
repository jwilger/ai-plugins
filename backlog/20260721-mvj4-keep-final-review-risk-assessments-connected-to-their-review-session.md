---
title: Keep final-review risk assessments connected to their review session
blocked_by: []
blocks: []
tags: [development-discipline, final-review, bug]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Prevent valid final reviews from failing because the risk assessment identity no longer matches the review session that requested it.

## Context / Why

A final_review.plan call in a separate repository rejected an apparently unchanged risk-scout result with `risk_assessment_assignment_id_mismatch=true`. The supplied assessment included its assignment ID, final-review-scoped subagent key, shared evidence identity, and caller attestation. Investigate how assignment identity is issued, retained, and compared across assess/attest/plan calls; make same-session results reliable while preserving rejection of stale or cross-session assessments. Diagnostics must remain actionable without exposing private review state.

## Acceptance criteria

## Subtasks

## Notes / Log
