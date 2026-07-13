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

## Subtasks

## Notes / Log
