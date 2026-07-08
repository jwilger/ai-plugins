---
title: Add cross-project worktree safety guidance
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Teach agents to detect coordination checkouts versus feature worktrees, respect repository worktree policies before editing, and avoid leaving dirty main checkouts when changes merely mirror upstream.

## Context / Why

## Acceptance criteria

- [ ] worktrees guidance tells agents to check whether the current checkout is a coordination checkout or feature worktree before making edits when a repo advertises a worktree policy.
- [ ] Guidance covers detecting dirty working trees that match upstream after fetch and avoiding redundant no-op changes in the main checkout.
- [ ] The change includes eval cases where an agent chooses a linked worktree for implementation and explains upstream-equivalent dirty state without compounding it.

## Subtasks

## Notes / Log
