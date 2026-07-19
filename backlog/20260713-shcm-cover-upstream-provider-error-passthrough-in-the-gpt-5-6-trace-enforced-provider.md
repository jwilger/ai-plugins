---
title: Cover upstream provider-error passthrough in the GPT-5.6 trace-enforced provider
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, tests, promptfoo, isolation, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add focused regression coverage proving the GPT-5.6 trace-enforced Promptfoo provider preserves an existing upstream provider error unchanged instead of replacing it with a trace-verification error.

## Context / Why

Implementation notes:\n\nDeferred MINOR from the fresh review of 20260709-spx8. The implementation already returns upstream provider errors before inspecting raw traces, but the focused isolation fixtures cover only nominally successful responses. This is test-depth hardening and does not block the current wrapper correction.

## Acceptance criteria

- [ ] A focused provider test supplies an upstream response with an existing error and no raw trace, then asserts the exact error response is returned unchanged.

## Subtasks

## Notes / Log
