---
title: Block unrelated work after a CI failure until its cause is repaired
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Add development-discipline guardrails that treat GitHub Actions as an authoritative delivery signal: once a pushed commit fails CI, agents must inspect the exact failed job and step, record the causal diagnosis, and make the next pushed commit a directly related repair before resuming unrelated implementation.

## Context / Why

## Acceptance criteria

- [x] A failed pushed CI run blocks unrelated implementation and pushes until its exact failed job and step are diagnosed.
- [x] The next pushed commit after a failure must state and address the diagnosed cause, or explicitly document why the failure is unrelated or transient with evidence.
- [ ] Workflow guidance requires polling the replacement commit to a terminal successful state before claiming the failure repaired.

## Subtasks

## Notes / Log

- 2026-07-19: Positive example from ticket 20260714-24xa: a full-marketplace canary returned a nonzero result, but inspection showed both harnesses named every loaded plugin and Codex accurately described eval-case-reporter. The actual defect was a prompt/checker mismatch requiring a literal skill name even though the prompt allowed a capability description. Correct behavior was to say the failure was not a plugin-loading failure, file the eval-harness defect separately, and avoid mixing that unrelated fix into the active final-review contract ticket. Use this as a concrete example of evidence-based failure classification and scope containment.
