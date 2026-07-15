---
title: Preserve review-budget timestamp relationships in parity normalization
blocked_by: []
blocks: []
tags: [development-discipline, release-parity, tests, bug]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Preserve the equality pattern of runtime review-budget start timestamps while normalizing cross-run clock drift in development-discipline release parity.

## Context / Why

Formal final review of ticket 20260715-yvha found that replacing every started_at_epoch_seconds value with zero masks a packaged binary that resets the review budget between transitions. Canonicalize distinct timestamps by first occurrence so cross-run absolute values disappear while within-transcript equality relationships remain observable. Add a focused two-record regression before changing the normalizer. This task blocks 20260715-yvha.

## Acceptance criteria

- [ ] The parity normalizer removes absolute clock drift while preserving the equality pattern of started_at_epoch_seconds values within a transcript.
- [ ] Focused coverage fails when source retains a review-budget start time but the distribution resets it between records.
- [ ] The focused release suite, release-from-source parity gate, and full repository CI pass at the fixed commit.

## Subtasks

## Notes / Log
