---
title: Fix hanging provider-backed behavior evals
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

## Context / Why

## Acceptance criteria

- [ ] Case and provider filters produce the intended scoped run, with tests proving EVAL_CASE_FILTER and EVAL_PROVIDER_FILTER do not unexpectedly expand to long serial suites.
- [ ] Provider-backed eval runs complete or fail with a bounded timeout instead of hanging silently in Claude Agent SDK or Codex SDK calls.

## Subtasks

## Notes / Log
