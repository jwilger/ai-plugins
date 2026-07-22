---
title: Keep final review risk assessments linked to the correct review
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Prevent final review from rejecting a valid risk assessment because its assignment identity is lost or compared against the wrong review.

## Context / Why

A final review can produce a detailed risk assessment and then reject that same assessment during planning with an assignment-identity mismatch. This interrupts delivery even though the assessment belongs to the current review, forcing repeated setup and creating uncertainty about whether evidence was accepted. The workflow should preserve and verify the assessment identity consistently from assignment through planning: accept a valid assessment from the current review, reject stale or cross-review assessments, and provide a safe diagnostic showing which identities conflicted. This matters because reliable review handoffs keep delivery moving without weakening protection against mixing evidence from different reviews. Implementation notes: reproduce the development-discipline final_review.plan failure where an assessment containing assignment_id risk-dda91c9e49f36832 was rejected with risk_assessment_assignment_id_mismatch=true. Investigate serialization, coordinator restart and recovery, assignment regeneration, and the identity comparison between assess_risk output, risk-scout submission, and plan input.

## Acceptance criteria

- [ ] Identity-mismatch errors report enough sanitized expected-versus-received information to diagnose the failure.
- [ ] An assessment from a different or stale final-review assignment remains rejected.
- [ ] Automated regression tests cover the reported planning failure and coordinator restart or recovery paths that can change assignment state.
- [ ] A risk assessment produced and submitted for the current final-review assignment is accepted by the planning step.

## Subtasks

## Notes / Log

- 2026-07-22: Observed again in the lanyard-ssh-agent 1.0.0 production-readiness final review: final_review.plan rejected the current detailed risk assessment with `risk_assessment_assignment_id_mismatch=true` (received assignment `risk-dda91c9e49f36832`). Treat this as a cross-project delivery-blocking regression case and preserve it in automated coverage.
