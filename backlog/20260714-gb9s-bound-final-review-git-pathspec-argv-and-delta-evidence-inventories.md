---
title: Bound final-review Git pathspec argv and delta-evidence inventories
blocked_by: []
blocks: []
tags: [development-discipline, final-review, reliability, scalability, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Chunk or stream final-review Git pathspecs and compact oversized changed-path inventories so valid maximum-size review scopes cannot exceed OS argv limits or the coordinator's 128 KiB evidence envelope.

## Context / Why

MINOR finding from the risk-proportionate final-review lightweight pass. The current implementation fails closed and MAX_CHANGED_FILES bounds exposure, so likelihood and impact are low; address when working next on snapshot-delta scalability.

## Acceptance criteria

## Subtasks

## Notes / Log
