---
title: Commit green increments and gate follow-up work on build state
blocked_by: []
blocks: []
tags: [development-discipline, workflow, git, ci, final-review, policy, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Update development-discipline guidance so each implementation increment is committed and pushed after fast unit tests and lightweight review pass, longer checks may run in CI, and full review gates only the claim that a ticket is complete.

## Context / Why

The current workflow held a large green change uncommitted through repeated full-review passes, increasing recovery risk and delaying CI feedback. The desired inner loop is: run the fast unit tests, run lightweight review, commit and push, then repeat until implementation is finished. Longer-running integration, mutation, full-suite, or similarly expensive checks may run only in CI rather than blocking each local increment. Run full review only at the ticket-completion boundary; if full review finds issues, fix them and again pass the fast unit-test/light-review gate, commit, and push before restarting full review. Before addressing full-review findings or starting another ticket, check the build for the latest pushed commit: running or green permits work; failed blocks follow-up work until the failure is understood and resolved. Full-review baselines must be pinned to the pre-ticket commit so incremental pushes to main do not erase or move the reviewed scope.

## Acceptance criteria

- [ ] Guidance defines full review as the ticket-completion gate, not a prerequisite for preserving a green increment.
- [ ] When full review finds issues, guidance requires a new green tests/light-review commit and push before restarting full review.
- [ ] Before addressing review findings or starting another ticket, guidance requires checking the latest pushed build; running or green permits work, while failed blocks follow-up work until resolved.
- [ ] Full-review instructions pin the baseline commit so pushes during review do not move or erase the reviewed diff.
- [ ] Guidance defines fast unit tests plus lightweight review as the local commit-and-push gate for each implementation increment.

## Subtasks

## Notes / Log
