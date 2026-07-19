---
title: Block unrelated work after a CI failure until its cause is repaired
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add development-discipline guardrails that treat GitHub Actions as an authoritative delivery signal: once a pushed commit fails CI, agents must inspect the exact failed job and step, record the causal diagnosis, and make the next pushed commit a directly related repair before resuming unrelated implementation.

## Context / Why

## Acceptance criteria

- [ ] A failed pushed CI run blocks unrelated implementation and pushes until its exact failed job and step are diagnosed.
- [ ] The next pushed commit after a failure must state and address the diagnosed cause, or explicitly document why the failure is unrelated or transient with evidence.

## Subtasks

## Notes / Log
