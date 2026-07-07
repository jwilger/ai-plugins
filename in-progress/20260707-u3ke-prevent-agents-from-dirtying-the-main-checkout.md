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
