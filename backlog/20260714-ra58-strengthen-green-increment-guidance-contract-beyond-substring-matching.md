---
title: Test development guidance for meaning, not just matching phrases
blocked_by: []
blocks: []
tags: [development-discipline, tests, skills, final-review, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen the fast policy checks so contradictory instructions cannot pass merely because they contain expected words. Tests should verify the intended relationship among small verified commits, CI failures, acceptance evidence, and follow-up review.

## Context / Why

MINOR test-verification finding from the risk-proportionate review-policy ticket. The semantic behavior fixture covers the intended workflow, but the fast static Bats gate could accept text such as 'never commit and push' or 'ignore a failed build' because it checks phrase presence only. It also does not structurally bind expensive suites to CI, actual acceptance criteria to full-review start, or delta assessment to affected-only reruns. Value: stronger policy regression detection. Risk/impact: low because runtime behavior is prose and a semantic fixture exists. Likelihood: unlikely but plausible during later editing. Opportunity cost: keep below observed product/tooling defects and address when strengthening skill-eval coverage.

## Acceptance criteria

- [ ] Reject negated or contradictory green-increment, CI-failure, and full-review guidance rather than accepting disconnected phrase presence.
- [ ] Bind expensive-suite deferral to CI, actual acceptance criteria to full-review start, and post-fix delta assessment to affected-only reruns.
- [ ] Keep a fast local contract and a semantic behavior fixture with calibrated pass and fail examples.

## Subtasks

## Notes / Log
