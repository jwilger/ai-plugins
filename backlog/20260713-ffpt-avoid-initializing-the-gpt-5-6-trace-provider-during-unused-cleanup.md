---
title: Do not start an unused GPT-5.6 provider during cleanup
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, promptfoo, process-lifecycle, tests, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make cleanup a true no-op when the GPT-5.6 provider was never used. Stopping an unused provider should not start a server process solely so it can be stopped.

## Context / Why

Deferred MINOR from the pinned-source isolation review of 20260709-spx8. The concurrency MAJOR requires promise-guarded initialization; the remaining cleanup path still calls the initializer when no evaluation call ever started. This wastes startup work but does not compromise current isolation or correctness.

## Acceptance criteria

- [ ] A focused lifecycle test calls cleanup before callApi and proves the inner provider loader and app-server startup are never invoked.

## Subtasks

## Notes / Log
