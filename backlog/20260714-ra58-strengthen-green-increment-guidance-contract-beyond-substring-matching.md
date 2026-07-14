---
title: Strengthen green-increment guidance contract beyond substring matching
blocked_by: []
blocks: []
tags: [development-discipline, tests, skills, final-review, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the green-increment workflow contract reject contradictory or inverted guidance instead of checking only disconnected required phrases.

## Context / Why

MINOR test-verification finding from the risk-proportionate review-policy ticket. The semantic behavior fixture covers the intended workflow, but the fast static Bats gate could accept text such as 'never commit and push' or 'ignore a failed build' because it checks phrase presence only. It also does not structurally bind expensive suites to CI, actual acceptance criteria to full-review start, or delta assessment to affected-only reruns. Value: stronger policy regression detection. Risk/impact: low because runtime behavior is prose and a semantic fixture exists. Likelihood: unlikely but plausible during later editing. Opportunity cost: keep below observed product/tooling defects and address when strengthening skill-eval coverage.

## Acceptance criteria

## Subtasks

## Notes / Log
