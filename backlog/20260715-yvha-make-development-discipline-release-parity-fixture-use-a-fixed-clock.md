---
title: Make development-discipline release parity deterministic across clock drift
blocked_by: [20260715-3kkp-label-parity-normalization-failures-by-side-and-record, 20260715-8hsq-preserve-zero-iteration-transition-differences-in-parity-normalization, 20260715-i2h6-preserve-review-budget-timestamp-relationships-in-parity-normalization, 20260715-s8xp-make-the-home-unset-development-discipline-launcher-test-provide-trusted-bash, 20260715-w7ga-scope-parity-clock-normalization-by-review-session]
blocks: [20260712-4qmz-reject-option-tokens-consumed-as-missing-tiber-option-values-before-writes]
tags: [development-discipline, tests, release, hermeticity, ci, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the source-versus-bundled development-discipline MCP parity check deterministic by normalizing only its known runtime-derived review-state fields before comparison.

## Context / Why

The canonical `just ci` gate compares complete JSON-RPC outputs from sequential source and bundled MCP runs. Each run captures the wall-clock second in `risk_plan.review_budget.started_at_epoch_seconds`; crossing a one-second boundary changes that field and the derived `review_contract_id`, causing `development-discipline-release-parity-mismatch=true` even when both binaries behave identically. The failure reproduced twice while Tiber-only gates remained green. The safer fixture design leaves the production clock and contract derivation untouched, normalizing only those exact runtime-derived fields in the parity comparison while preserving unrelated differences and failing closed on malformed JSONL.

## Acceptance criteria

- [ ] The release-from-source parity gate passes repeatedly when source and bundled behavior is identical while still failing for a real output difference.
- [ ] Focused coverage reproduces the former clock-derived mismatch and proves the deterministic fixture contract.
- [ ] The parity fixture leaves production clock behavior unchanged and normalizes only the exact runtime-derived review-state fields needed to compare sequential runs.

## Subtasks

## Notes / Log
