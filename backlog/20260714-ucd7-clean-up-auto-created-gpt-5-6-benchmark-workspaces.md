---
title: Remove temporary GPT-5.6 benchmark workspaces after every run
blocked_by: []
blocks: []
tags: [minor, evals, cleanup, workspace, operability]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Clean up only the temporary workspaces created automatically by the benchmark runner, whether a run succeeds, fails, times out, or is interrupted. Never remove a workspace that the user configured explicitly.

## Context / Why

Implementation notes:\n\nVerified MINOR production footgun from 20260709-spx8: each live run reaching workspace preparation leaves an ai-plugins-gpt56-workspace-* directory and marker in TMPDIR because the runner has no cleanup trap.

## Acceptance criteria

- [ ] Successful, failed, timed-out, and interrupted runs clean an auto-created default workspace, while GPT56_BENCHMARK_WORKSPACE overrides are never removed.

## Subtasks

## Notes / Log

- 2026-07-14: Scope boundary from 20260713-uf3e split: this ticket exclusively owns cleanup of runner-owned default workspaces on success, failure, timeout, and interruption. The replacement split tickets do not duplicate cleanup.
