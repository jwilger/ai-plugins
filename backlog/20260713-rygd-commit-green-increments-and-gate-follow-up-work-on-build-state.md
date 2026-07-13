---
title: Commit green increments and gate follow-up work on build state
blocked_by: []
blocks: []
tags: [development-discipline, workflow, git, ci, final-review, policy, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Update development-discipline guidance so each implementation increment is committed and pushed after tests and lightweight review pass, while full review gates only the claim that a ticket is complete.

## Context / Why

The current workflow held a large green change uncommitted through repeated full-review passes, increasing recovery risk and delaying CI feedback. The desired cadence is: commit and push every green, lightly reviewed increment; run full review only at ticket-completion boundaries; if full review finds issues, fix them and again commit/push once tests and lightweight review pass before restarting full review. Before addressing full-review findings or starting another ticket, check the build for the latest pushed commit: running or green permits work; failed blocks follow-up work until the failure is understood and resolved. Full-review baselines must be pinned to the pre-ticket commit so incremental pushes to main do not erase or move the reviewed scope.

## Acceptance criteria

- [ ] Guidance defines tests plus lightweight review as the commit-and-push gate for each implementation increment.
- [ ] Guidance defines full review as the ticket-completion gate, not a prerequisite for preserving a green increment.

## Subtasks

## Notes / Log
