---
title: Make GPT-5.6 benchmark preflight and artifacts phase-safe
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, artifacts, process-lifecycle, operability, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Resolve benchmark paths once, keep execution and grader-calibration artifacts distinct, and ensure preparation failure or interruption cannot leave stale evidence appearing current.

## Context / Why

Split from 20260713-uf3e. Ambient output overrides can collapse execution and grader calibration into the same paths, relative overrides can resolve differently across phases, and Codex-home preparation currently occurs before the canonical lifecycle clears or marks prior artifacts and installs signal handling. This task owns path resolution, phase separation, and stale-evidence-safe preparation; it does not own diagnostic wording, canonical exit-status precedence, provider locking, timeout policy, or default-workspace cleanup.

## Acceptance criteria

- [ ] Execution and grader calibration always resolve to distinct artifact directories, including under every supported output-root override.
- [ ] Relative GPT56_BENCHMARK_OUT_ROOT and EVAL_OUT_DIR overrides resolve once against the caller's original working directory, and preparation, canonical execution, and post-run checking use the same absolute paths.

## Subtasks

## Notes / Log
