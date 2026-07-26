---
name: worktrees
description: Use when setting up worktree support, routing feature edits away from a coordination checkout, creating linked worktrees, or diagnosing worktree policy.
---

# Worktrees

Read `[features].worktrees` and `[worktrees]` from
`.development-system.toml`.

Setup and initialization must run from the primary checkout. Feature edits must
run from a linked worktree when worktrees are enabled. The primary checkout is
the coordination checkout, not a feature workspace.

Before editing, compare Git's absolute `--git-dir` and `--git-common-dir`.
Equality identifies the primary checkout; inequality identifies a linked
worktree. Never require a linked worktree for setup.
