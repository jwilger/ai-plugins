---
title: Prevent stale or mixed-up GPT-5.6 benchmark results
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, artifacts, process-lifecycle, operability, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Resolve benchmark paths consistently, keep execution and grading results separate, and ensure setup failures or interruptions cannot make old artifacts look like current evidence.

## Context / Why

Implementation notes: Split from 20260713-uf3e. Ambient output overrides can collapse execution and grader calibration into the same paths, relative overrides can resolve differently across phases, and Codex-home preparation currently occurs before the canonical lifecycle clears or marks prior artifacts and installs signal handling. This task owns path resolution, phase separation, and stale-evidence-safe preparation; it does not own diagnostic wording, canonical exit-status precedence, provider locking, timeout policy, or default-workspace cleanup.

## Acceptance criteria

- [ ] Execution and grader calibration always resolve to distinct artifact directories, including under every supported output-root override.
- [ ] Relative GPT56_BENCHMARK_OUT_ROOT and EVAL_OUT_DIR overrides resolve once against the caller's original working directory, and preparation, canonical execution, and post-run checking use the same absolute paths.
- [ ] Preparation is covered by the signal-aware lifecycle or clears and marks stale phase artifacts before work begins, so preparation failure or interruption cannot be reported as a completed provider comparison.
- [ ] Focused regressions distinguish successful preparation, preparation failure, and preparation interruption and prove that no stale result is accepted as fresh evidence.

## Subtasks

## Notes / Log

- 2026-07-14: Split from 20260713-uf3e. This ticket exclusively owns phase-safe path resolution, artifact separation, preparation lifecycle, and stale-evidence prevention.
