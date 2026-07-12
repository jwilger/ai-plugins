---
title: Add bounded verification guidance to development-discipline
blocked_by: []
blocks: []
tags: [development-discipline, verification, ci, reliability]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add general bounded-verification guidance for long tests, evals, CI, and external checks, including evidence-based waiting, hang diagnosis, cancellation, and fallback-claim rules.

## Context / Why

Promptfoo now has repository-specific timeouts, but development-discipline still lacks portable policy for long or broken verifiers. Merge the CI-specific task 20260712-i76j here: a normally running gate remains waiting rather than blocked; use a comparable recent successful run as the baseline, and when the current run exceeds it by roughly five minutes without a plausible workload explanation, inspect the active step and logs before deliberately deciding whether to cancel and retry. A timeout or verifier failure never counts as success, and fallback evidence must narrow the completion claim.

## Acceptance criteria

- [ ] development-discipline verification guidance requires bounded timeouts or explicit monitoring plans for long-running tests, evals, CI checks, and external tools.
- [ ] Guidance tells agents to track broken verification infrastructure separately instead of treating an unbounded hang as permanent completion evidence or rediscovering it every review cycle.
- [ ] The change includes eval cases covering a hanging verifier and the expected bounded-timeout or tracked-blocker response.
- [ ] Every long-running check has an explicit timeout or monitoring/cancellation plan; an unbounded hang or timeout never counts as passing evidence.
- [ ] CI waiting uses a comparable recent successful duration as its baseline, treats ordinary pending work as waiting, and inspects the active step/logs after roughly five unexplained minutes beyond that baseline before considering cancellation.
- [ ] Timeout and hang records include the command or check, elapsed time, active step or last output, retained artifacts, and a stable blocker reference so the same infrastructure failure is carried forward.
- [ ] Cancellation and retry are deliberate actions with clear authority, and fallback evidence explicitly limits any completion or readiness claim.

## Subtasks

## Notes / Log

- 2026-07-12: Backlog grooming: canonical task now includes the CI wait/hang policy from abandoned duplicate 20260712-i76j.
