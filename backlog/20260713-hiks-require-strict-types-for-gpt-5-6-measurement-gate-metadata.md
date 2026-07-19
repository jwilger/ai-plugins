---
title: Reject incorrectly typed GPT-5.6 measurement settings
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, validation, types, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the measurement checker reject malformed values instead of letting automatic JavaScript conversion turn values such as an empty string or false into an accepted zero. Invalid artifacts should produce a clear diagnostic.

## Context / Why

Implementation notes:\n\nDeferred MINOR from the correctness review of 20260709-spx8. scripts/evals/check-thresholds.mjs currently uses Number(minPassRate) === 0, so values such as an empty string or false satisfy the measurement-mode contract. This does not invalidate correctly generated current artifacts, but it weakens malformed-artifact diagnostics and should be tightened outside the active migration ticket.

## Acceptance criteria

- [ ] Measurement-mode min_pass_rate is accepted only when it is a finite numeric value exactly equal to zero; strings, booleans, null, and non-finite numbers are rejected with an actionable diagnostic.
- [ ] Focused regression tests cover valid numeric zero and representative coercible malformed values without changing ordinary threshold-mode behavior.

## Subtasks

## Notes / Log
