---
title: Benchmark contract, inputs, cases, and locked fixture
blocked_by: []
blocks: []
tags: [evals, benchmark, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Review and independently disposition the benchmark declaration, input/case matrix, workspace preparation, validation, and locked Rust fixture extracted from the zcsh final-review scope split.

## Context / Why

Created from final-review session final-review-zcsh-final-20260718 at diff hash 54ece87326cb708bd8d46f1c25a3bbeeef69884a. Scope: benchmark README, benchmark-inputs.cjs, benchmark.json, cases.cjs, expense-report fixture, promptfooconfig.yaml, prepare-code-quality-workspaces.mjs, and validate-code-quality-contract.mjs. This slice is independently shippable because it defines a deterministic benchmark and prepared workspace matrix without trusted scoring or canonical runtime execution.

## Acceptance criteria

- [ ] Declare the exact independent condition/sample/case matrix, provider and prompt inputs, deterministic gates, and locked Rust fixture.

## Subtasks

## Notes / Log
