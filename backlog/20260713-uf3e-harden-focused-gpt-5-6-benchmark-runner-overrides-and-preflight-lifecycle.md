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
- [ ] An explicitly empty PROMPTFOO_MAX_CONCURRENCY is either documented as equivalent to unset or rejected distinctly; regression coverage locks the chosen contract.
- [ ] Relative GPT56_BENCHMARK_OUT_ROOT and EVAL_OUT_DIR overrides resolve once against the caller's original working directory, and preparation, the canonical runner, and post-run checkers use the same absolute paths.
- [ ] A successful focused live-run regression proves the shared provider lock and its inherited identity are held during both provider execution and post-run artifact checking, then released after the complete lifecycle.
- [ ] Dry-run regression coverage proves the focused wrapper neither acquires nor creates the provider lock.
- [ ] Focused and canonical runners canonicalize the shared provider-lock identity consistently when the checkout or runner is invoked through a symlink, with a regression covering the nested handoff.

## Subtasks

## Notes / Log

- 2026-07-13: 2026-07-13 review update: 20260709-spx8 now rejects concurrency values outside canonical 1-2, but ${PROMPTFOO_MAX_CONCURRENCY:-2} still treats an explicitly empty inherited value as default 2. Preserve this MINOR unset-vs-empty case in the follow-up.
- 2026-07-13: 2026-07-13 lightweight review of 20260709-spx8 deferred this caused MINOR test-depth finding: current contention coverage proves fail-fast behavior, but does not probe lock retention through the checker, release after completion, or an initially absent lock remaining absent during dry-run.
