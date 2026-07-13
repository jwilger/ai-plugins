---
title: Make the eval launch-race regression prove pre-publication signal handling
blocked_by: []
blocks: []
tags: [evals, signals, tests, race-condition, minor, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen the real-SIGINT launch-boundary regression so it proves the runner's signal trap executes while eval_pid is still unpublished, rather than allowing the test to pass through the ordinary published-PID path.

## Context / Why

A final-review MINOR found that the current BASH_ENV DEBUG-hook fixture pauses immediately before eval_pid="$!", sends SIGINT, and then creates capture.release without waiting for proof that the trap ran. Because signal delivery is asynchronous, the assignment may execute before trap dispatch, so the test can pass even if the eval_launching deferred-signal branch regresses. Add a deterministic trap-executed handshake while keeping the production runner free of test-only hooks.

## Acceptance criteria

## Subtasks

## Notes / Log
