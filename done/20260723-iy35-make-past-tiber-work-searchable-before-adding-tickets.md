---
title: Make past Tiber work searchable before adding tickets
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Backlog admission should compare a candidate with completed and previously rejected work, but Tiber's supported list surfaces expose only open tickets. Add a straightforward way to discover and search historical tickets before creating a new one.

## Context / Why

Without historical discovery, agents must know an old ticket identifier in advance or inspect the tasks Git branch directly. That makes duplicate prevention slow and unreliable. CLI and model-context-protocol users should be able to query completed and abandoned tickets by status and search their titles and product-facing descriptions, with stable structured results suitable for admission workflows.

## Acceptance criteria

- [ ] CLI and model-context-protocol users can list tickets by backlog, in-progress, done, or abandoned status without inspecting the tasks Git branch directly.
- [ ] Users can search historical ticket titles and product-facing descriptions with stable structured results that include task identity and status.
- [ ] Documentation and tests demonstrate using historical search to detect a completed or rejected duplicate before admission.

## Subtasks

## Notes / Log

- 2026-07-24: CI recovery: failed SHA 42503a87 run 30066235406 Quality gate Full gate at cargo fmt check; caused by unapplied rustfmt. Repair SHA 4ba47ed3 applied rustfmt only and focused tests passed. Replacement run 30066398123 completed successfully.
- 2026-07-24: Failure record: 42503a87f3f21638a9d88563640e60fb5db4d7df; run 30066235406; exact failed job=Quality gate; failed step=Full gate; relevant log evidence=cargo fmt --check printed diffs in status-list files.
- 2026-07-24: Diagnosis: status-list changes were pushed without rustfmt; classification=caused; supporting evidence=focused CLI and MCP tests passed after formatting and the bounded repair review found formatting-only changes.
- 2026-07-24: Next action: tested causal repair 4ba47ed3043418428ee7af365d21ee80c67744f4 whose commit body explains the failed run and cause. Release proof: replacement run 30066398123; terminal status=success; queued|pending|running=false.
- 2026-07-24: Final-review coordinator defect reproduced twice: final_review.plan rejected the exact returned risk assessment with risk_assessment_assignment_id_mismatch, including a fresh explicit session iy35-final-review-retry-1 and assignment risk-0dbe25488d2bf4a1. Bounded risk scout was clean/low, increment reviews were clean, tiber-rust and just ci passed, focused behavior eval thresholds passed, and release completeness passed.
