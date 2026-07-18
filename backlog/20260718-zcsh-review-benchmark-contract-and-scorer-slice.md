---
title: Review benchmark contract and scorer slice
blocked_by: []
blocks: []
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
