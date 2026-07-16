---
title: Make provider eval timeouts safely resumable without matrix expansion
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Preserve completed provider-backed eval work across watchdog timeouts and resume the exact remaining case/provider matrix without duplication.

## Context / Why

A full 276-case behavior eval hit the runner's 90-minute GNU timeout. The TERM/KILL path did not produce usable incremental outputs. Promptfoo 0.121.18 advertises --resume, but resuming this generated dynamic-provider eval reconstructed 36 prompt entries instead of 6 and announced 1,656 cases instead of 276. The malformed resume was paused with SIGINT, which did save state cleanly. The runner needs a repository-owned, fail-closed resume contract rather than relying on raw Promptfoo replay semantics.

## Acceptance criteria

- [ ] A timed-out or intentionally paused provider eval preserves a stable eval identifier and enough completed-row state to continue without rerunning successful provider calls.
- [ ] The runner exposes a documented resume workflow that retains the cross-worktree provider lock, isolated Codex homes, output ownership checks, and provider-composition validation.
- [ ] Resume validates the expected case/provider/sample identity and refuses to run if reconstruction changes the matrix cardinality or duplicates prompt/provider combinations.
- [ ] Regression tests reproduce the dynamic-matrix expansion risk and prove a 276-row run cannot silently become 1,656 rows on resume.
- [ ] Timeout shutdown prefers Promptfoo's graceful pause signal before bounded TERM/KILL escalation, with clear status and recovery diagnostics.

## Subtasks

## Notes / Log
