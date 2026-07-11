---
title: Block pushes until final-review completes three clean passes
blocked_by: []
blocks: []
tags: [development-discipline, final-review, worktrees, release-enforcement, bug]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Fix the release/push enforcement gap that allowed a branch to be pushed after only one started final-review pass rather than the required three completed clean iterations.

## Context / Why

The development-discipline final-review workflow correctly states that publication requires three consecutive clean iterations, but it does not currently provide a mechanical push gate that checks the authoritative review state. An agent started the review, received only partial first-iteration results, then pushed anyway. Add a proportionate enforcement mechanism linking the reviewed diff hash and branch state to pre-push/merge behavior so incomplete, stale, or mismatched review state blocks publication with clear remediation.

## Acceptance criteria

- [ ] A push or merge of an in-scope change is blocked unless the authoritative final-review state records three consecutive clean iterations for the exact current diff hash.

## Subtasks

## Notes / Log
