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

- [x] A failed pushed CI run blocks unrelated implementation and pushes until its exact failed job and step are diagnosed.
- [x] The next pushed commit after a failure must state and address the diagnosed cause, or explicitly document why the failure is unrelated or transient with evidence.
- [x] Workflow guidance requires polling the replacement commit to a terminal successful state before claiming the failure repaired.

## Subtasks

## Notes / Log

- 2026-07-19: Positive example from ticket 20260714-24xa: a full-marketplace canary returned a nonzero result, but inspection showed both harnesses named every loaded plugin and Codex accurately described eval-case-reporter. The actual defect was a prompt/checker mismatch requiring a literal skill name even though the prompt allowed a capability description. Correct behavior was to say the failure was not a plugin-loading failure, file the eval-harness defect separately, and avoid mixing that unrelated fix into the active final-review contract ticket. Use this as a concrete example of evidence-based failure classification and scope containment.
- 2026-07-20: Completed in bc5a1d5 and review repair 500726c. Full local CI passed: 579 Bats tests, 260 development-discipline unit tests, 44 mutation candidates (38 caught, 6 unviable), formatting, manifests, release provenance, and four binary checksums. Claude and Codex full-marketplace behavior evals passed for both the natural recovery scenario and exact recovery record; plugin-eval analysis scored 100/100. Formal final review completed clean with no findings or completion blockers. Exact GitHub Actions run 29711246772 for commit 500726cf868ec527b3bfcab436018dc9a6608ea4 reached terminal success, including Quality gate and CI gate.
