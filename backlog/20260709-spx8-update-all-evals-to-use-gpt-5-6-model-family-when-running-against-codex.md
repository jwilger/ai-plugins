---
title: Update all evals to use gpt-5.6 model family when running against Codex
blocked_by: []
blocks: []
tags: [evals, codex, model-routing, benchmarking]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Benchmark the GPT-5.6 family for Codex eval execution and grading roles, then migrate every Codex eval surface to an evidence-backed standard-run/grader split while retaining explicit overrides.

## Context / Why

The canonical Codex eval matrix, runner help, README, and semantic grader currently default to gpt-5.5. Evaluate GPT-5.6 Terra at medium reasoning as the standard-run candidate and GPT-5.6 Sol at high reasoning as the grader candidate, while verifying exact current identifiers and availability from official documentation before implementation. Representative advisor-like eval scenarios belong in the benchmark population, but this task does not select or configure the installed advisor plugin's agent; 20260711-wtk6 is the canonical fixed gpt-5.6-sol/high advisor routing decision. Measure quality, latency, and token/cost behavior and document whether the grader should remain independent from the model under test. Claude providers and their defaults are out of scope.

## Acceptance criteria

- [ ] The eval matrix, Codex provider configuration, semantic grader, runner help, README, generated-config tests, and dashboard/site labels are updated with no stale default gpt-5.5 references.
- [ ] Existing environment overrides remain supported and Claude provider configuration is unchanged.
- [ ] Generated-config and dry-run coverage passes, plus a focused provider-backed sample when credentials and approval are available; any unavailable live evidence is stated explicitly.
- [ ] A documented benchmark compares supported GPT-5.6 candidates on representative standard eval cases and advisor-like eval scenarios using pass rate, latency, token usage, and cost, after verifying exact current model identifiers.
- [ ] The selected Codex eval execution and grader model/reasoning split is justified from the benchmark, including grader independence; it does not alter the installed advisor agent governed by 20260711-wtk6.

## Subtasks

## Notes / Log
