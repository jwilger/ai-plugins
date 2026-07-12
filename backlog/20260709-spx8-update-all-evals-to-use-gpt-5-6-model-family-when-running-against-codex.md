---
title: Update all evals to use gpt-5.6 model family when running against Codex
blocked_by: []
blocks: []
tags: [evals, codex, model-routing, benchmarking]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Benchmark the GPT-5.6 family on representative Codex eval roles, then migrate every Codex execution and grading surface to an evidence-backed normal/advisor split while retaining explicit overrides.

## Context / Why

The canonical Codex eval matrix, runner help, README, and semantic grader currently default to gpt-5.5. Evaluate GPT-5.6 Terra at medium reasoning as the normal-run candidate and GPT-5.6 Sol at high reasoning as the grader/advisor candidate, while verifying exact current identifiers and availability from official documentation before implementation. Measure quality, latency, and token/cost behavior; document whether the grader should remain independent from the model under test. Claude providers and their defaults are out of scope.

## Acceptance criteria

## Subtasks

## Notes / Log
