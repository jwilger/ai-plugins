---
title: Keep large final reviews within operating-system and evidence-size limits
blocked_by: []
blocks: []
tags: [development-discipline, final-review, reliability, scalability, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Process large changed-file lists in bounded pieces so a valid maximum-size final review cannot exceed command-line or coordinator evidence limits. Preserve the same review scope and fail safely with actionable information.

## Context / Why

MINOR finding from the risk-proportionate final-review lightweight pass. The current implementation fails closed and MAX_CHANGED_FILES bounds exposure, so likelihood and impact are low; address when working next on snapshot-delta scalability.

## Acceptance criteria

## Subtasks

## Notes / Log
