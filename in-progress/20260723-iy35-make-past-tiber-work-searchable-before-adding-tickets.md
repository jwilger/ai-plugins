---
title: Make past Tiber work searchable before adding tickets
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
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
