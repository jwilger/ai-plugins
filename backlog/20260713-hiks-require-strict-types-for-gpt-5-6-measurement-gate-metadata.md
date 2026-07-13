---
title: Require strict types for GPT-5.6 measurement-gate metadata
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, validation, types, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the focused GPT-5.6 measurement gate reject malformed metadata instead of accepting values that JavaScript numeric coercion turns into zero.

## Context / Why

Deferred MINOR from the correctness review of 20260709-spx8. scripts/evals/check-thresholds.mjs currently uses Number(minPassRate) === 0, so values such as an empty string or false satisfy the measurement-mode contract. This does not invalidate correctly generated current artifacts, but it weakens malformed-artifact diagnostics and should be tightened outside the active migration ticket.

## Acceptance criteria

- [ ] Measurement-mode min_pass_rate is accepted only when it is a finite numeric value exactly equal to zero; strings, booleans, null, and non-finite numbers are rejected with an actionable diagnostic.
- [ ] Focused regression tests cover valid numeric zero and representative coercible malformed values without changing ordinary threshold-mode behavior.

## Subtasks

## Notes / Log
