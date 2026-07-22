---
title: Preserve original provider errors in GPT-5.6 evaluations
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, tests, promptfoo, isolation, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a focused test proving that when the upstream evaluation provider already reports an error, the trace-checking wrapper returns that error unchanged instead of replacing it with a secondary trace-validation failure.

## Context / Why

Implementation notes:\n\nDeferred MINOR from the fresh review of 20260709-spx8. The implementation already returns upstream provider errors before inspecting raw traces, but the focused isolation fixtures cover only nominally successful responses. This is test-depth hardening and does not block the current wrapper correction.

## Acceptance criteria

- [ ] A focused provider test supplies an upstream response with an existing error and no raw trace, then asserts the exact error response is returned unchanged.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
