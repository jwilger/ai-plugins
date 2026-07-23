---
title: Make past Tiber work searchable before adding tickets
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Backlog admission should compare a candidate with completed and previously rejected work, but Tiber's supported list surfaces expose only open tickets. Add a straightforward way to discover and search historical tickets before creating a new one.

## Context / Why

Without historical discovery, agents must know an old ticket identifier in advance or inspect the tasks Git branch directly. That makes duplicate prevention slow and unreliable. CLI and model-context-protocol users should be able to query completed and abandoned tickets by status and search their titles and product-facing descriptions, with stable structured results suitable for admission workflows.

## Acceptance criteria

- [ ] CLI and model-context-protocol users can list tickets by backlog, in-progress, done, or abandoned status without inspecting the tasks Git branch directly.
- [ ] Users can search historical ticket titles and product-facing descriptions with stable structured results that include task identity and status.

## Subtasks

## Notes / Log
