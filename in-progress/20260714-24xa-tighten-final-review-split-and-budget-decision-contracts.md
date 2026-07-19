---
title: Tighten final-review split and budget decision contracts
blocked_by: []
blocks: []
tags: [development-discipline, final-review, mcp, contracts, guardrails, major, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Prevent final-review split contracts from accepting overlapping, synthetic, recursive, or post-landing delivery decompositions, and align budget-decision schemas with runtime validation.

## Context / Why

Originally tracked overlapping split scopes and budget schema parity. Expanded after a real incident where a broad already-landed diff produced a scope_split_hold, the agent created top-level Tiber review blockers and path-filtered remote branches, and each artificial child recursively split again. Final review must distinguish review batching from delivery decomposition, model landed/unlanded lifecycle and split lineage, require genuine independent build/test/shipping evidence, and stop before mutating a tracker when the proposed split is recursive or administrative.

## Acceptance criteria

- [x] Reject or explicitly model split candidates whose scope ownership is fully overlapping, while preserving collective coverage of the changed-file inventory.
- [x] Make final_review.advance's review_budget_decision JSON Schema accept exactly the payload shapes and bounds accepted by runtime validation.
- [x] Add focused tests for overlapping split scopes and schema/runtime boundary parity.
- [x] Reject recursive split holds for a child of the same root work item and source diff; return a bounded guardrail/tool-policy result instead of more split candidates.
- [x] Treat already-landed broad scopes as retrospective review work: broadness may batch review internally but must not authorize delivery-ticket decomposition without concrete unfinished work.
- [x] Require split candidates to prove genuine independent build, test, and shipping boundaries; path coverage alone or synthetic path-filtered scopes are insufficient.
- [x] Require an explicit caller confirmation step before a split plan may be represented as tracker tickets or blocking dependencies.
- [x] Update the final-review skill guidance to forbid synthetic/pushed review-only branches, recursive split-ticket creation, and use of Tiber blocks for administrative review coordination.
- [ ] Add focused regression tests for landed scope handling, recursive split lineage, synthetic/path-only candidates, and confirmation gating.

## Subtasks

## Notes / Log
