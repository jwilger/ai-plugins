---
title: Clean up auto-created GPT-5.6 benchmark workspaces
blocked_by: []
blocks: []
tags: [minor, evals, cleanup, workspace, operability]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Remove only runner-owned PID-unique default GPT-5.6 workspaces on every exit while preserving explicitly configured workspaces.

## Context / Why

Verified MINOR production footgun from 20260709-spx8: each live run reaching workspace preparation leaves an ai-plugins-gpt56-workspace-* directory and marker in TMPDIR because the runner has no cleanup trap.

## Acceptance criteria

- [ ] Successful, failed, timed-out, and interrupted runs clean an auto-created default workspace, while GPT56_BENCHMARK_WORKSPACE overrides are never removed.

## Subtasks

## Notes / Log

- 2026-07-14: Scope boundary from 20260713-uf3e split: this ticket exclusively owns cleanup of runner-owned default workspaces on success, failure, timeout, and interruption. The replacement split tickets do not duplicate cleanup.
