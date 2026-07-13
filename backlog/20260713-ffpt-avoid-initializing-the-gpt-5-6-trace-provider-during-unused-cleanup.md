---
title: Avoid initializing the GPT-5.6 trace provider during unused cleanup
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, promptfoo, process-lifecycle, tests, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make cleanup on an unused GPT-5.6 trace-enforced Promptfoo provider a no-op instead of lazily starting an app-server process solely to stop it.

## Context / Why

Deferred MINOR from the pinned-source isolation review of 20260709-spx8. The concurrency MAJOR requires promise-guarded initialization; the remaining cleanup path still calls the initializer when no evaluation call ever started. This wastes startup work but does not compromise current isolation or correctness.

## Acceptance criteria

- [ ] A focused lifecycle test calls cleanup before callApi and proves the inner provider loader and app-server startup are never invoked.

## Subtasks

## Notes / Log
