---
title: Add production-risk and footgun lenses to engineering-standards
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen engineering-standards so agents routinely look for sneaky footguns and dev/test-safe patterns that fail under expected production load, burst traffic, DOS-like pressure, or partial failure.

## Context / Why

## Acceptance criteria

- [ ] engineering-standards guidance explicitly reviews for hidden footguns, unsafe defaults, partial failure states, unbounded retries, unbounded loops, lock contention, cache staleness, and cleanup hazards.
- [ ] Guidance explicitly asks whether data access patterns, N+1 work, fanout, memory/IO growth, and thundering-herd behavior will survive production-sized use or DOS-like bursts.

## Subtasks

## Notes / Log
