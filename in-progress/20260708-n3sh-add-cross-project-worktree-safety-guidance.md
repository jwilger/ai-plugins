---
title: Help agents avoid feature work in a repository’s coordination checkout
blocked_by: []
blocks: []
tags: [worktrees, guidance, developer-experience, evals]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Extend reusable worktree guidance so agents recognize when a repository reserves its main checkout for coordination, distinguish real local changes from files that already match upstream, and move feature work to the correct linked worktree without disturbing user changes.

## Context / Why

The repo-local agent checkout guard already detects coordination checkouts and upstream-equivalent dirty state, while the reusable worktrees plugin mainly guards commit/push. Teach the portable workflow to apply only when a repository advertises a coordination-checkout policy, inspect Git dir versus common dir before edits, preserve genuine user changes, distinguish clean/local-dirty/upstream-equivalent states, and route feature work to a linked worktree. Mechanical integration enforcement remains in 20260711-qhgu.

## Acceptance criteria

- [x] worktrees guidance tells agents to check whether the current checkout is a coordination checkout or feature worktree before making edits when a repo advertises a worktree policy.
- [ ] Guidance covers detecting dirty working trees that match upstream after fetch and avoiding redundant no-op changes in the main checkout.
- [ ] The change includes eval cases where an agent chooses a linked worktree for implementation and explains upstream-equivalent dirty state without compounding it.
- [ ] Guidance applies only when repository policy advertises a coordination checkout, preserves existing user changes, and documents any explicit exception boundary.
- [ ] Diagnostics distinguish clean, genuinely locally dirty, and upstream-equivalent dirty coordination states and give an exact linked-worktree or no-op remediation without compounding the main checkout.

## Subtasks

## Notes / Log
