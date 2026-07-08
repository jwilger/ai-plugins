---
title: Fix hanging provider-backed behavior evals
blocked_by: []
blocks: []
tags: []
pr_mr_url: https://github.com/jwilger/ai-plugins/pull/44
pr_mr_status: merged
---

## Summary

## Context / Why

## Acceptance criteria

- [x] Case and provider filters produce the intended scoped run, with tests proving EVAL_CASE_FILTER and EVAL_PROVIDER_FILTER do not unexpectedly expand to long serial suites.
- [x] Provider-backed eval runs complete or fail with a bounded timeout instead of hanging silently in Claude Agent SDK or Codex SDK calls.

## Subtasks

## Notes / Log

- 2026-07-08: Observed while preparing the Tiber new-task PR: EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture hung in a Claude Agent SDK call after expanding to 60 cases; EVAL_PROVIDER_FILTER=codex-gpt-5.5 then hung in an OpenAI Codex SDK call after expanding to 30 cases. Both had to be interrupted and produced paused promptfoo eval IDs rather than usable behavior evidence.
