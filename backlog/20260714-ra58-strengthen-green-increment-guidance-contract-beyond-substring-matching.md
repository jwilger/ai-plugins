---
title: Test real behavior instead of repository text
blocked_by: []
blocks: []
tags: [development-discipline, tests, test-quality, high-priority, guardrails]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Tests should prove behavior that users or callers can observe. Tests that only open a committed file and check whether it contains a particular phrase are brittle and do not prove that the product works. Tests that inspect continuous integration workflow files have the same problem because running those workflows is already the meaningful test. Establish this as a standing rule for every project, and remove or replace existing tests that violate it when they are found.

## Context / Why

MINOR test-verification finding from the risk-proportionate review-policy ticket. The semantic behavior fixture covers the intended workflow, but the fast static Bats gate could accept text such as 'never commit and push' or 'ignore a failed build' because it checks phrase presence only. It also does not structurally bind expensive suites to CI, actual acceptance criteria to full-review start, or delta assessment to affected-only reruns. Value: stronger policy regression detection. Risk/impact: low because runtime behavior is prose and a semantic fixture exists. Likelihood: unlikely but plausible during later editing. Opportunity cost: keep below observed product/tooling defects and address when strengthening skill-eval coverage.

## Acceptance criteria

- [ ] Development guidance clearly forbids tests that only read committed repository files and check for specific strings, except when testing a program that creates or edits that file.

## Subtasks

## Notes / Log
