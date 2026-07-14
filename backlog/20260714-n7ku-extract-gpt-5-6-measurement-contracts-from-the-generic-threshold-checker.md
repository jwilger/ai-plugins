---
title: Extract GPT-5.6 measurement contracts from the generic threshold checker
blocked_by: []
blocks: []
tags: [minor, evals, architecture, maintainability, promptfoo]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Extract the Promptfoo 0.121.18/GPT-5.6 measurement source and artifact contract from the repository-wide threshold CLI into a focused pure module.

## Context / Why

Verified MINOR architecture finding from 20260709-spx8: scripts/evals/check-thresholds.mjs now mixes roughly 900 lines of benchmark-specific contract validation and module-scoped measurement state with generic pass-rate/value-gate behavior, increasing regression coupling for future Promptfoo upgrades or additional benchmarks.

## Acceptance criteria

- [ ] A focused pure module owns source/artifact normalization and returns structured contract failures; check-thresholds.mjs remains a thin composition layer and generic-mode regression coverage stays green.

## Subtasks

## Notes / Log
