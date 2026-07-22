---
title: Preserve the correct failure result from GPT-5.6 calibration runs
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, runner, exit-status, tests, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add tests proving the focused calibration command reports the original benchmark failure when appropriate, while still running its independent artifact checks. Timeouts and crashes must not be misreported as ordinary missing-result failures.

## Context / Why

Implementation notes:\n\nCaused MINOR deferred from 20260709-spx8. The current fake Promptfoo exits 7, but canonical scripts/evals/run.sh converts that to generic checker status 1 on the deliberately failed artifact, so the test does not distinguish canonical status precedence from custom-checker failure. Build a controlled no-artifact/distinctive-status fixture and separately prove checker invocation and returned-status precedence.

## Acceptance criteria

- [ ] For ordinary nonzero execution outcomes with a readable artifact, the focused execution phase still runs the independent isolation checker and preserves the canonical runner status.
- [ ] Both focused phases classify timeout status 124 and every signal/crash status at or above 128 as terminal and skip misleading post-run missing-artifact checks.

## Subtasks

## Notes / Log

- 2026-07-14: Scope boundary from 20260713-uf3e split: this ticket owns canonical status precedence and terminal 124/128+ classification; 20260714-2xyd owns artifact freshness and preparation lifecycle.
- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
