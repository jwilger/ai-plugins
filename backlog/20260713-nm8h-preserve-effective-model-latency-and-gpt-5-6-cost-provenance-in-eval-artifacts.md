---
title: Preserve effective model, latency, and GPT-5.6 cost provenance in eval artifacts
blocked_by: []
blocks: []
tags: [evals, benchmarking, observability, gpt-5.6, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make eval artifacts and the static dashboard report the actual model and reasoning effort used after overrides, retain latency and grader provenance, and calculate GPT-5.6 token/credit cost accurately.

## Context / Why

Discovered while migrating canonical Codex eval defaults to GPT-5.6. Provider labels currently come from the static matrix variant even when CODEX_EVAL_MODEL overrides the effective model; build-site drops latency and does not expose grader model/effort; Promptfoo 0.121.17 has no GPT-5.6 billing entries, so its cost can be missing or zero. These are pre-existing observability limitations and do not block the focused migration benchmark, which will use explicit model labels and calculate documented Codex credit rates separately.

## Acceptance criteria

- [ ] Every persisted result records the resolved execution model and reasoning effort, and provider/dashboard labels remain accurate when environment overrides are used.
- [ ] Result summaries retain and render execution latency, token usage, and separate grader model/effort provenance.
- [ ] GPT-5.6 Sol, Terra, and Luna costs use current documented Codex credit rates (or an explicitly versioned source), with tests preventing missing or zero cost from being presented as real spend.

## Subtasks

## Notes / Log
