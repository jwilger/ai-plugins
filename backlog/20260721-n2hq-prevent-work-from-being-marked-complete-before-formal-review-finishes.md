---
title: Prevent work from being marked complete before formal review finishes
blocked_by: []
blocks: []
tags: [development-discipline, final-review, guardrail, evals, high-priority]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Keep work visibly incomplete until every required formal review check is submitted and the review reaches a successful final result. This prevents an earlier lightweight review from being mistaken for completed formal review.

## Context / Why

GitHub issue 57 documents a case where automated checks and a lightweight review passed, but formal review requested more reviewers. Those reviews were never run, yet the work was reported as fully reviewed. Once a formal review session exists, completion should require that session to reach a successful final state. Missing or interrupted review state must count as incomplete, and evaluation coverage should reject skipped assignments followed by completion claims.

## Acceptance criteria

- [ ] Work cannot be reported complete or ready while a formal review has required checks that are still outstanding.
- [ ] The authoritative review state distinguishes automated checks, lightweight review, and completed formal review.

## Subtasks

## Notes / Log
