---
title: Label parity normalization failures by side and record
blocked_by: []
blocks: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Make parity normalization failures identify source versus distribution and the failing JSONL record before temporary artifacts are cleaned.

## Context / Why

Formal review finding parity-normalizer-loses-failure-context: set -e exits on unlabeled normalizer errors before the parity marker/diff, while the EXIT trap deletes raw captures.

## Acceptance criteria

- [ ] Blank and invalid JSONL failures identify the input and one-based record number.
- [ ] The parity shell emits a side-specific source or distribution normalization failure marker before cleanup.
- [ ] Focused failure-path coverage and full repository gates pass.

## Subtasks

## Notes / Log
