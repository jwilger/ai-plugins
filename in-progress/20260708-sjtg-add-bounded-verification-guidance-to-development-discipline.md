---
title: Set clear limits for long-running verification
blocked_by: []
blocks: []
tags: [development-discipline, verification, ci, reliability]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Add reusable guidance for monitoring long tests, evaluations, CI builds, and external checks. Agents should know when to keep waiting, when to investigate a possible hang, when cancellation is appropriate, and how incomplete verification limits any readiness claim.

## Context / Why

Implementation notes: Promptfoo now has repository-specific timeouts, but development-discipline still lacks portable policy for long or broken verifiers. Merge the CI-specific task 20260712-i76j here: a normally running gate remains waiting rather than blocked; use a comparable recent successful run as the baseline, and when the current run exceeds it by roughly five minutes without a plausible workload explanation, inspect the active step and logs before deliberately deciding whether to cancel and retry. A timeout or verifier failure never counts as success, and fallback evidence must narrow the completion claim.

## Acceptance criteria

- [x] Guidance tells agents to track broken verification infrastructure separately instead of treating an unbounded hang as permanent completion evidence or rediscovering it every review cycle.
- [x] The change includes eval cases covering a hanging verifier and the expected bounded-timeout or tracked-blocker response.
- [ ] Every long-running check has an explicit timeout or monitoring/cancellation plan; an unbounded hang or timeout never counts as passing evidence.
- [ ] CI waiting uses a comparable recent successful duration as its baseline, treats ordinary pending work as waiting, and inspects the active step/logs after roughly five unexplained minutes beyond that baseline before considering cancellation.
- [ ] Timeout and hang records include the command or check, elapsed time, active step or last output, retained artifacts, and a stable blocker reference so the same infrastructure failure is carried forward.
- [ ] Cancellation and retry are deliberate actions with clear authority, and fallback evidence explicitly limits any completion or readiness claim.

## Subtasks

## Notes / Log

- 2026-07-12: Backlog grooming: canonical task now includes the CI wait/hang policy from abandoned duplicate 20260712-i76j.
- 2026-07-21: Delivered in commits 2fa6b738080a0b0d194b8cc5007349f050a62267 and f840bfb51da4a175e2451176ef0bcb37464c10e8. Semantic provider-backed evals passed for hanging verification (2/2: Claude and Codex) and unusually slow CI (6/6: three Claude and three Codex compositions). Full local `nix develop -c just ci` passed (586 Bats tests, 0 failures; mutation testing 38 caught, 6 unviable). Final review of the pinned diff bf7ed8a..f840bfb completed clean. Exact pushed GitHub Actions run 29844145210 reached terminal success; Quality gate completed in 22m44s.
