---
title: Review benchmark contract and scorer slice
blocked_by: [20260719-nvz3-runtime-provenance-result-validation-and-secret-safe-publication, 20260719-zk46-nix-runtime-toolchain-and-focused-integration-surface]
blocks: [20260715-n6bs-build-writable-downstream-code-quality-benchmarks-for-codex-plugins]
tags: [evals, codex, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Independently review and ship the downstream benchmark contract, trusted scorer, replay fixtures, and result classification required by 20260715-n6bs.

## Context / Why

Split from 20260715-n6bs after formal final review returned scope_split_hold for an unusually broad new subsystem. This slice covers benchmark cases, metric/taxonomy contracts, trusted scoring and replay evidence, without the credential/runtime boundary or repository integration slice.

## Acceptance criteria

- [ ] Benchmark metrics and outcome taxonomy are predeclared and mechanically enforced.
- [ ] Trusted scoring and regression replay detect incomplete, duplicate, unexpected, and provenance-mismatched results.
- [ ] Diagnostic evidence is allowlisted, bounded, and secret-scanned.
- [ ] All three benchmark conditions are represented and scored independently.

## Subtasks

## Notes / Log

- 2026-07-19: Final-review session final-review-zcsh-final-20260718 entered authoritative scope_split_hold for baseline 351cf031939d460e88b4d6f3e37f297a6bfc01df, diff hash 54ece87326cb708bd8d46f1c25a3bbeeef69884a, and shared evidence zcsh-final-tests (67/67). Blocking split tickets: 20260719-cb43 contract/fixture; 20260719-qpx4 trusted verifier/scorer (including direct duplicate raw-result rejection coverage); 20260719-nvz3 provenance/results/secret-safe publication; 20260719-zk46 Nix runtime integration. No deep-review assignments are permitted on zcsh until these blockers are independently dispositioned.
