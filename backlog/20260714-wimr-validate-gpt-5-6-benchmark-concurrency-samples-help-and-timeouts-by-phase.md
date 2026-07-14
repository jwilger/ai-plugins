---
title: Validate GPT-5.6 benchmark concurrency, samples, help, and timeouts by phase
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, validation, timeouts, concurrency, cli, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Give the focused benchmark wrapper strict phase-aware contracts for concurrency, sample overrides, help, and effective deadlines.

## Context / Why

Split from 20260713-uf3e. Inherited concurrency can exceed the supported maximum, explicitly empty values have no clear contract, ambient invalid concurrency can block help, stale sample overrides can affect the wrong phase, and the fixed timeout does not scale across the supported sample range. This task owns option validation and effective deadline calculation, not provider locking, artifact freshness, canonical exit-status precedence, or sample-expansion and label-matrix test depth.

## Acceptance criteria

- [ ] PROMPTFOO_MAX_CONCURRENCY accepts only the documented supported range, and an explicitly empty value is either documented as equivalent to unset or rejected distinctly.
- [ ] Canonical runner help remains available despite an invalid ambient concurrency override, while live and dry-run execution validate before preparation or launch.

## Subtasks

## Notes / Log
