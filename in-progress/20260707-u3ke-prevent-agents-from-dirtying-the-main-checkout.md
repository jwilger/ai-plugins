---
title: Prevent agents from dirtying the main checkout
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

Agents working from the coordination checkout can leave origin-equivalent or generated changes in the main worktree. Recent example: after fetch, the dirty files matched origin/main so applying them was a no-op, but the main checkout still appeared modified and behind. Define and enforce a workflow that keeps the main checkout clean, preferably by doing feature work in repo-local linked worktrees and detecting main-checkout mutations before completion.

## Context / Why

## Acceptance criteria

- [ ] Starting agent work from the main checkout does not leave modified or untracked project files unless the user explicitly requested main-checkout edits.
- [ ] The workflow distinguishes real user changes from origin-equivalent generated changes and documents the expected cleanup or worktree handoff behavior.

## Subtasks

## Notes / Log

- 2026-07-07: Implemented locally on branch agent-main-checkout-guard at commit 73b96fc. Verification run: nix develop -c just bats validate-marketplace. Awaiting explicit approval to push/open PR.
- 2026-07-07: PR publication gate: pushing agent-main-checkout-guard to origin was rejected by escalation policy until the user explicitly approves publishing the branch to GitHub.
- 2026-07-07: PR #33 opened. CI failed on the new upstream-equivalent Bats case because the fixture bare remote HEAD was unset in CI; fixed with signed follow-up commit 56c1b74 and pushed.
