---
title: Add bounded verification guidance to development-discipline
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Teach agents that long-running verification, evals, and external checks must have bounded timeouts, useful diagnostics, and a fallback evidence policy when the verifier itself is broken or hanging.

## Context / Why

## Acceptance criteria

- [ ] development-discipline verification guidance requires bounded timeouts or explicit monitoring plans for long-running tests, evals, CI checks, and external tools.
- [ ] Guidance tells agents to track broken verification infrastructure separately instead of treating an unbounded hang as permanent completion evidence or rediscovering it every review cycle.
- [ ] The change includes eval cases covering a hanging verifier and the expected bounded-timeout or tracked-blocker response.

## Subtasks

## Notes / Log
