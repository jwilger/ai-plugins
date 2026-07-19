---
title: Benchmark contract, inputs, cases, and locked fixture
blocked_by: [20260719-62jw-isolated-workspace-matrix-preparation, 20260719-m65h-runtime-provenance-and-regular-tree-hashing, 20260719-pe3u-locked-expense-report-benchmark-fixture, 20260719-zcj7-benchmark-contract-and-provider-input-surface]
blocks: [20260718-zcsh-review-benchmark-contract-and-scorer-slice]
tags: [evals, benchmark, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Review and independently disposition the benchmark declaration, input/case matrix, workspace preparation, validation, and locked Rust fixture extracted from the zcsh final-review scope split.

## Context / Why

Created from final-review session final-review-zcsh-final-20260718 at diff hash 54ece87326cb708bd8d46f1c25a3bbeeef69884a. Scope: benchmark README, benchmark-inputs.cjs, benchmark.json, cases.cjs, expense-report fixture, promptfooconfig.yaml, manifest.cjs, runtime-manifest.cjs, prepare-code-quality-workspaces.mjs, validate-code-quality-contract.mjs, and the shared code-quality-tree-hash.mjs dependency. The three shared provenance utilities overlap the provenance ticket because direct import checks proved they are required to execute this contract slice in isolation. This slice defines a deterministic benchmark and prepared workspace matrix without trusted scoring or canonical runtime execution.

## Acceptance criteria

- [ ] Declare the exact independent condition/sample/case matrix, provider and prompt inputs, deterministic gates, and locked Rust fixture.
- [ ] Reject malformed, duplicate, incomplete, or inconsistent workspace matrices and contract values.
- [ ] Complete final review against the isolated slice with current diff-bound verification evidence.

## Subtasks

## Notes / Log

- 2026-07-19: Isolation check found prepare-code-quality-workspaces.mjs imports code-quality-tree-hash.mjs. Added that shared utility to this review scope (overlapping provenance ownership) so the slice is actually executable in isolation; no production code changed.
- 2026-07-19: Import check additionally found cases.cjs requires manifest.cjs and runtime-manifest.cjs. Added both shared contract utilities to the isolated scope; this matches the earlier broader contract split and prevents a non-executable review artifact.
- 2026-07-19: Final-review session final-review-cb43-20260719 entered scope_split_hold at baseline 351cf031939d460e88b4d6f3e37f297a6bfc01df and diff hash b190d81690f3657f5230580fb083b666e86c8237. Scout required four further independently shippable slices: contract/provider surface, locked expense fixture, workspace matrix preparation, and runtime provenance/tree hashing.
