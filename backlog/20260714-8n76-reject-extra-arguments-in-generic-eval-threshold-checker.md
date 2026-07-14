---
title: Reject extra arguments in generic eval threshold checker
blocked_by: []
blocks: []
tags: [evals, cli, compatibility, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Preserve strict CLI behavior after the GPT-5.6 measurement checker split by rejecting unsupported trailing arguments instead of silently ignoring them.

## Context / Why

Lightweight behavior review of the GPT-5.6 measurement-contract extraction found that scripts/evals/check-thresholds.mjs now consumes only argv[2]. Before the split, its shared parser rejected unknown, missing-value, and duplicate options with exit status 2. The supported production path is unaffected, so this is a deferred MINOR compatibility and diagnostics improvement.

## Acceptance criteria

- [ ] scripts/evals/check-thresholds.mjs exits with status 2 when arguments follow the required results JSON path.

## Subtasks

## Notes / Log
