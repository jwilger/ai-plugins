---
title: Show In Progress before Backlog on single-column Tiber boards
blocked_by: []
blocks: []
tags: [tiber, dashboard, responsive-ui, usability, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Order responsive Tiber board columns so In Progress is the first visible column when the dashboard collapses to a single-column layout.

## Context / Why

On a narrow screen the dashboard currently renders Backlog first, forcing users to scroll past the backlog to find active work. Preserve the normal multi-column workflow while making the single-column reading order start with In Progress.

## Acceptance criteria

- [ ] At the single-column responsive breakpoint, the In Progress column is rendered before Backlog.
- [ ] At wider multi-column breakpoints, the intended board layout and column ordering remain unchanged.
- [ ] Responsive UI coverage verifies the narrow-screen column order and guards against regression.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Real Tiber enhancement or edge case, but lower pain, severity, frequency, or leverage than closure correctness and non-destructive setup. The reserved product slot covers backlog-limit enforcement; this item is rejected with no hidden queue.
