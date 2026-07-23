---
title: Resume final reviews without losing prior work
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Large final-review responses can be truncated or become difficult to recover, forcing an agent to restart work that was already completed. Preserve a compact, retrievable review record so interrupted sessions can continue from the last valid state.

## Context / Why

Final review is a required delivery gate, and repeating it wastes time while increasing the chance of inconsistent decisions. The durable record should retain the review identity, lifecycle state, evidence references, assigned lenses, accepted findings, and next permitted transition without reproducing enormous tool payloads.

## Acceptance criteria

- [ ] A session can retrieve compact final-review state after truncation or compaction and continue with the correct next transition without restarting the review.
- [ ] Recovered state preserves review identity, lifecycle, evidence references, assigned lenses, findings, and completed transitions without storing an oversized duplicate payload.

## Subtasks

## Notes / Log

- 2026-07-23: Rejected during backlog admission review as duplicative. Recent final-review work already persists session state, supports restart recovery, and commits reports with session transitions; no distinct unmet product outcome was established.
