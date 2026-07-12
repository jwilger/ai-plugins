---
title: Add cross-project worktree safety guidance
blocked_by: []
blocks: []
tags: [worktrees, guidance, developer-experience, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Extend portable worktree guidance with upstream-equivalent dirty-main diagnosis and no-op edit avoidance, complementing the separate mechanical guard task.

## Context / Why

The repo-local agent checkout guard already detects coordination checkouts and upstream-equivalent dirty state, while the reusable worktrees plugin mainly guards commit/push. Teach the portable workflow to apply only when a repository advertises a coordination-checkout policy, inspect Git dir versus common dir before edits, preserve genuine user changes, distinguish clean/local-dirty/upstream-equivalent states, and route feature work to a linked worktree. Mechanical integration enforcement remains in 20260711-qhgu.

## Acceptance criteria

- [ ] worktrees guidance tells agents to check whether the current checkout is a coordination checkout or feature worktree before making edits when a repo advertises a worktree policy.
- [ ] Guidance covers detecting dirty working trees that match upstream after fetch and avoiding redundant no-op changes in the main checkout.
- [ ] The change includes eval cases where an agent chooses a linked worktree for implementation and explains upstream-equivalent dirty state without compounding it.

## Subtasks

## Notes / Log
