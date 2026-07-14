---
title: Preserve effective model, latency, and GPT-5.6 cost provenance in eval artifacts
blocked_by: []
blocks: []
tags: [evals, benchmarking, observability, gpt-5.6, codex, dashboard, model-overrides, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make eval artifacts and dashboards report the actual resolved execution model and reasoning effort after overrides, retain latency and grader provenance, and calculate GPT-5.6 token or credit cost accurately.

## Context / Why

Discovered while migrating canonical Codex eval defaults to GPT-5.6. Provider labels currently come from the static matrix variant even when CODEX_EVAL_MODEL overrides the effective model; build-site drops latency and does not expose grader model or effort; Promptfoo may lack complete GPT-5.6 billing entries, so cost can be missing or zero. Consolidated scope from 20260713-j3x4 requires the effective-model contract to prevent a static Terra provider ID or label from surviving when CODEX_EVAL_MODEL selects Sol or Luna. Persisted artifacts, aggregation, and the rendered dashboard must agree on the effective model.

## Acceptance criteria

- [ ] Every persisted result records the resolved execution model and reasoning effort, and provider/dashboard labels remain accurate when environment overrides are used.
- [ ] Result summaries retain and render execution latency, token usage, and separate grader model/effort provenance.
- [ ] GPT-5.6 Sol, Terra, and Luna costs use current documented Codex credit rates (or an explicitly versioned source), with tests preventing missing or zero cost from being presented as real spend.
- [ ] Focused config, artifact, aggregation, and dashboard regressions cover CODEX_EVAL_MODEL overriding the configured Terra default to both Sol and Luna and prove that no static provider ID or label continues to report Terra.

## Subtasks

## Notes / Log

- 2026-07-14: Backlog grooming 2026-07-14: Consolidated 20260713-j3x4 here because effective override labeling is a strict subset of this ticket's model/provenance contract. Its Sol/Luna override regression is now explicit.
