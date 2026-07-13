---
title: Harden focused GPT-5.6 benchmark runner overrides and preflight lifecycle
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, process-management, operability, safety, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the focused GPT-5.6 benchmark wrapper enforce safe concurrency, preserve phase-specific artifacts under overrides, and report preflight interruption or failure without stale evidence.

## Context / Why

Deferred MINOR findings from final review of 20260709-spx8. An inherited PROMPTFOO_MAX_CONCURRENCY can exceed the documented maximum of two; an ambient EVAL_OUT_DIR can collapse execution and grader-calibration into the same artifact paths; and Codex-home preparation currently occurs before the canonical runner installs signal handling and clears or marks prior artifacts.

## Acceptance criteria

- [ ] The focused wrapper rejects or caps concurrency overrides outside a documented safe range, and tests cover inherited values.
- [ ] Execution and grader-calibration always resolve to distinct artifact directories, including when supported output-root overrides are present.
- [ ] Preflight preparation is covered by the signal-aware lifecycle or clears and marks stale phase artifacts before work begins.
- [ ] Focused regressions distinguish preparation failure or interruption from a completed provider comparison and prevent stale results from being treated as fresh evidence.

## Subtasks

## Notes / Log

- 2026-07-13: 2026-07-13 review update: 20260709-spx8 now rejects concurrency values outside canonical 1-2, but ${PROMPTFOO_MAX_CONCURRENCY:-2} still treats an explicitly empty inherited value as default 2. Preserve this MINOR unset-vs-empty case in the follow-up.
