---
title: Prove canonical status precedence in the GPT-5.6 calibration runner
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, runner, exit-status, tests, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a regression that proves the focused calibration wrapper preserves a distinctive ordinary canonical-runner failure status while still invoking and observing the custom artifact checker.

## Context / Why

Caused MINOR deferred from 20260709-spx8. The current fake Promptfoo exits 7, but canonical scripts/evals/run.sh converts that to generic checker status 1 on the deliberately failed artifact, so the test does not distinguish canonical status precedence from custom-checker failure. Build a controlled no-artifact/distinctive-status fixture and separately prove checker invocation and returned-status precedence.

## Acceptance criteria

- [ ] For ordinary nonzero execution outcomes with a readable artifact, the focused execution phase still runs the independent isolation checker and preserves the canonical runner status.
- [ ] Both focused phases classify timeout status 124 and every signal/crash status at or above 128 as terminal and skip misleading post-run missing-artifact checks.

## Subtasks

## Notes / Log

- 2026-07-14: Scope boundary from 20260713-uf3e split: this ticket owns canonical status precedence and terminal 124/128+ classification; 20260714-2xyd owns artifact freshness and preparation lifecycle.
