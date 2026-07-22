---
title: Make restarted final reviews recover reliably
blocked_by: []
blocks: []
tags: [bug, development-discipline, high-priority]
pr_mr_url: 
pr_mr_status: 
---

## Summary

A restarted final review can reject the exact review assignment it just created, leaving an otherwise ready change unable to proceed. Make valid restarts continue and give clear recovery guidance when review state is stale or belongs to another session.

## Context / Why

Final review is a required delivery safeguard. When its restart process cannot continue, legitimate work stops even when the documented steps were followed. A fresh review should accept its own current assignment when the reviewed change and evidence match. Genuine stale or mismatched state should produce sanitized, actionable recovery instructions. Implementation notes: cover final_review.assess_risk and final_review.plan session binding, assignment consumption, caller-carried state, and abandoned-session recovery. Source: GitHub issue #58.

## Acceptance criteria

- [ ] A restarted review accepts the assignment returned by its matching risk assessment when the session, reviewed change, and evidence are current.
- [ ] A stale, consumed, or session-mismatched assignment is rejected with sanitized details that identify the mismatch.
- [ ] The rejection tells the caller how to restart, resume, or abandon the review safely.
- [ ] Automated tests reproduce GitHub issue #58 and prove both the successful restart and each supported recovery path.

## Subtasks

## Notes / Log

- 2026-07-21: Triaged from GitHub issue #58: https://github.com/jwilger/ai-plugins/issues/58
- 2026-07-22: 2026-07-22 curation: Combined K9C6 because both describe the same cross-project delivery-blocking root cause: final-review assignment identity can be lost or mismatched across restart, assess_risk, and plan. Preserve K9C6 reproduction risk_assessment_assignment_id_mismatch=true plus sanitized expected-versus-received diagnostics.
