---
title: Cover GPT-5.6 benchmark sample overrides and exact provider-label contracts
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, tests, benchmarking, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen the focused GPT-5.6 benchmark loader/config regression coverage for repeated samples and the complete Sol/Terra/Luna provider-label matrix.

## Context / Why

Deferred MINOR from the fresh-context review of 20260709-spx8. The current regression proves the representative case categories, exact advisor-like workloads, direct/no-Advisor wording, and advisor-like provider suffix. It does not independently prove GPT56_BENCHMARK_SAMPLES expansion or enumerate all six unique standard/advisor-like Sol/Terra/Luna labels. This does not block the one-sample migration benchmark.

## Acceptance criteria

- [ ] A focused test proves GPT56_BENCHMARK_SAMPLES=2 produces exactly two uniquely indexed samples for every benchmark case while the unset default remains one.
- [ ] A focused test enumerates exactly six unique Sol/Terra/Luna provider labels across standard and advisor-like modes and verifies each case uses the correct three-label subset.
- [ ] A focused measurement-gate test proves an entirely absent configured case reports every missing provider/sample tuple, including the zero-result artifact edge case.

## Subtasks

## Notes / Log

- 2026-07-13: Deferred MINOR from the measurement-completeness rereview of 20260709-spx8: current coverage proves config-derived omitted-case rejection and the shared provider/sample completeness loop, but does not assert exhaustive missing-tuple diagnostics for a wholly absent case or the zero-result artifact edge. This is test-depth hardening only; existing behavior already fails closed.
