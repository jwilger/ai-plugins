---
title: Make development-discipline release parity deterministic across clock drift
blocked_by: [20260715-3kkp-label-parity-normalization-failures-by-side-and-record, 20260715-8hsq-preserve-zero-iteration-transition-differences-in-parity-normalization, 20260715-i2h6-preserve-review-budget-timestamp-relationships-in-parity-normalization, 20260715-s8xp-make-the-home-unset-development-discipline-launcher-test-provide-trusted-bash, 20260715-w7ga-scope-parity-clock-normalization-by-review-session]
blocks: [20260712-4qmz-reject-option-tokens-consumed-as-missing-tiber-option-values-before-writes]
tags: [development-discipline, tests, release, hermeticity, ci, major, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Make the source-versus-bundled development-discipline MCP parity check deterministic by normalizing only its known runtime-derived review-state fields before comparison.

## Context / Why

The canonical `just ci` gate compares complete JSON-RPC outputs from sequential source and bundled MCP runs. Each run captures the wall-clock second in `risk_plan.review_budget.started_at_epoch_seconds`; crossing a one-second boundary changes that field and the derived `review_contract_id`, causing `development-discipline-release-parity-mismatch=true` even when both binaries behave identically. The failure reproduced twice while Tiber-only gates remained green. The safer fixture design leaves the production clock and contract derivation untouched, normalizing only those exact runtime-derived fields in the parity comparison while preserving unrelated differences and failing closed on malformed JSONL.

## Acceptance criteria

- [x] The release-from-source parity gate passes repeatedly when source and bundled behavior is identical while still failing for a real output difference.
- [x] Focused coverage reproduces the former clock-derived mismatch and proves the deterministic fixture contract.
- [x] The parity fixture leaves production clock behavior unchanged and normalizes only the exact runtime-derived review-state fields needed to compare sequential runs.

## Subtasks

## Notes / Log

- 2026-07-15: Implemented across commits 07b6c942, da1c9863, 7027d09b, 9e11f9ad, 1ffdf41f, cfa46bd2, and 1bb7e27. The normalizer removes cross-run clock/derived-ID drift while preserving session-scoped timestamp equality, contract/transition relationships, malformed and noncanonical state, zero/non-safe iterations, unsafe JSON numbers, and real output differences. Diagnostics fail closed with side+record markers before cleanup and redact malformed payloads. Evidence: focused release/startup tests and the real source-versus-distribution gate pass; `nix develop -c just ci` passes with 242 development-discipline tests, mutation testing 38 caught/6 unviable, and all 360 Bats tests. Formal final review `parity-final-cfa46bd-v5` completed at diff fced6e7d840829974ce5027d20f9c7d7786f2ff6 with no blockers or out-of-scope findings. A confirmation review rejected changing zero timestamps because production explicitly uses 0 as its clock-error fallback and the timestamp contract intentionally preserves equality relationships rather than absolute values.
- 2026-07-15: Pushed directly to main at 1bb7e27b21554d1906351801a444a03e0f828e29. Exact trunk GitHub Actions run 29410714463 completed green: Quality gate, Eval config dry-run, Codex cross-harness manifests, and aggregate CI gate all succeeded.
