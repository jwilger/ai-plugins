---
title: Test all GPT-5.6 sample and provider-label combinations
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, tests, benchmarking, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Expand benchmark tests to cover repeated samples, every Sol, Terra, and Luna label in standard and advisor-like modes, and complete missing-result diagnostics. This should catch misleading or incomplete benchmark reports.

## Context / Why

Implementation notes:\n\nDeferred MINOR from the fresh-context review of 20260709-spx8. The current regression proves the representative case categories, exact advisor-like workloads, direct/no-Advisor wording, and advisor-like provider suffix. It does not independently prove GPT56_BENCHMARK_SAMPLES expansion or enumerate all six unique standard/advisor-like Sol/Terra/Luna labels. This does not block the one-sample migration benchmark.

## Acceptance criteria

- [ ] A focused test proves GPT56_BENCHMARK_SAMPLES=2 produces exactly two uniquely indexed samples for every benchmark case while the unset default remains one.
- [ ] A focused test enumerates exactly six unique Sol/Terra/Luna provider labels across standard and advisor-like modes and verifies each case uses the correct three-label subset.
- [ ] A focused measurement-gate test proves an entirely absent configured case reports every missing provider/sample tuple, including the zero-result artifact edge case.

## Subtasks

## Notes / Log

- 2026-07-13: Deferred MINOR from the measurement-completeness rereview of 20260709-spx8: current coverage proves config-derived omitted-case rejection and the shared provider/sample completeness loop, but does not assert exhaustive missing-tuple diagnostics for a wholly absent case or the zero-result artifact edge. This is test-depth hardening only; existing behavior already fails closed.
- 2026-07-14: Scope boundary from 20260713-uf3e split: this ticket owns repeated-sample expansion, exact provider labels, and missing-tuple diagnostics. 20260714-wimr owns production validation and effective-timeout semantics.
- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
