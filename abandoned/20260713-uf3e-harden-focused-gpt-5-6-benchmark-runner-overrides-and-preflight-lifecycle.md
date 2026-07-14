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
- [ ] PID-named default benchmark workspaces are safely removed on exit after ownership-marker verification, while explicitly supplied workspaces are preserved.
- [ ] GPT56_BENCHMARK_SAMPLES validation applies only to execution; grader calibration remains runnable when an irrelevant stale or malformed sample override is present.
- [ ] Canonical eval-runner help remains available even when ambient PROMPTFOO_MAX_CONCURRENCY is invalid; live/dry-run validation still occurs before preparation or launch.
- [ ] Supported GPT56_BENCHMARK_SAMPLES values either receive a benchmark-specific effective timeout that scales with the documented turn count, or require an explicit EVAL_TIMEOUT with a preflight diagnostic; dry-run prints the effective deadline.
- [ ] The cross-worktree provider lock lives outside disposable .dependencies caches, fails closed when a checkout's Git-common location cannot be resolved, and regression coverage proves unlink/recreation cannot create a second concurrently acquirable lock.

## Subtasks

## Notes / Log

- 2026-07-13: 2026-07-13 review update: 20260709-spx8 now rejects concurrency values outside canonical 1-2, but ${PROMPTFOO_MAX_CONCURRENCY:-2} still treats an explicitly empty inherited value as default 2. Preserve this MINOR unset-vs-empty case in the follow-up.
- 2026-07-13: 2026-07-13 lightweight review of 20260709-spx8 deferred this caused MINOR test-depth finding: current contention coverage proves fail-fast behavior, but does not probe lock retention through the checker, release after completion, or an initially absent lock remaining absent during dry-run.
- 2026-07-13: 2026-07-13 follow-up lightweight review of 20260709-spx8 deferred a caused MINOR: the focused runner exports a logical checkout lock path while run.sh canonicalizes only the inherited side, so symlink-invoked nested runs can reject their own held lock with status 75.
- 2026-07-13: 20260709-spx8 final review deferred a caused MINOR: live runs using the PID-named default workspace leave it behind. Covered by the ownership-checked default-workspace cleanup criterion.
- 2026-07-14: 2026-07-13 formal review of 20260709-spx8 verified two additional caused MINOR follow-ups: the fixed 20-minute focused timeout does not scale across the supported 1-10 sample range, and the cross-worktree provider lock can split when its disposable .dependencies path is removed or Git-common resolution fails open.
- 2026-07-14: 2026-07-14 formal final-review pass 1 for 20260709-spx8 reconfirmed that the supported 1-10 sample range still inherits a fixed 20-minute timeout. This remains covered by the existing timeout-scaling acceptance criterion; deferred as MINOR without changing the frozen diff.
- 2026-07-14: Backlog grooming 2026-07-14: Split this unusually broad ticket into 20260714-2xyd for path, artifact, and preflight lifecycle; 20260714-g89d for provider-lock identity and lifetime; and 20260714-wimr for concurrency, sample, help, and timeout contracts. Existing 20260714-ucd7 exclusively owns default-workspace cleanup. Existing 20260713-h9bn owns canonical status precedence, 20260713-hkrf owns sample expansion, provider-label, and missing-tuple test depth, and 20260713-g56q owns setup-versus-provider diagnostic wording. All original acceptance criteria are preserved without leaving this monolith executable.
