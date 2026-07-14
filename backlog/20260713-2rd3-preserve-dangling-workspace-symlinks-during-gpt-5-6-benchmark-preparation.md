---
title: Harden GPT-5.6 benchmark workspace ownership and overlap handling
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, filesystem, safety, symlinks, ownership, tests, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Preserve dangling or unowned symlinks, require the exact ownership marker before recursive recreation, and retain realpath-aware bidirectional overlap protection for benchmark workspaces and credential homes.

## Context / Why

Final review of 20260709-spx8 found that a dangling workspace symlink is treated as absent and replaced. Related deferred test-depth findings show that current regressions do not prove exact marker contents or the complete realpath-aware ancestor, descendant, symlink-alias, explicit-auth, and default ~/.codex overlap matrix. Consolidate these tightly coupled workspace deletion-authorization and isolation requirements into one bounded filesystem-safety increment.

## Acceptance criteria

- [ ] A dangling workspace symlink is refused without removing or replacing the symlink or its target.

## Subtasks

## Notes / Log

- 2026-07-14: Reconfirmed independently in the final lightweight review for 20260709-spx8: a dangling symlink is treated as missing and replaced. No current-ticket diff change was made because this remains a deferred MINOR.
- 2026-07-14: 2026-07-14 formal final-review pass 1 for 20260709-spx8 independently reconfirmed the dangling-symlink ownership gap. Deferred as MINOR; the frozen source diff is unchanged.
