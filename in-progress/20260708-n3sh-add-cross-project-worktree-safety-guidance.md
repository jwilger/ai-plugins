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
- [x] Guidance covers detecting dirty working trees that match upstream after fetch and avoiding redundant no-op changes in the main checkout.
- [x] The change includes eval cases where an agent chooses a linked worktree for implementation and explains upstream-equivalent dirty state without compounding it.
- [x] Guidance applies only when repository policy advertises a coordination checkout, preserves existing user changes, and documents any explicit exception boundary.
- [x] Diagnostics distinguish clean, genuinely locally dirty, and upstream-equivalent dirty coordination states and give an exact linked-worktree or no-op remediation without compounding the main checkout.

## Subtasks

## Notes / Log

- 2026-07-22: Delivered to main at d1247259bd3555b00ed5edec9ecf407d23343c53. Verification: local just ci passed (44 mutants; 570 Bats); focused worktrees matrix eval-iJJ-2026-07-22T21:06:37 met all thresholds with 16/16 plugin-enabled rows passing; clean-state remediation eval-tmz-2026-07-22T21:45:54 passed all four plugin-enabled variants with both no-plugin baselines failing as expected; final review completed at diff 5ddbd9918f52143c81a2551efdb00b5859c35d86 with no blockers or out-of-scope findings; GitHub CI run 29960651838 completed successfully, including the CI gate.
