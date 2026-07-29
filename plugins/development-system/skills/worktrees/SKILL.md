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
worktree. When explaining a workflow that combines setup and feature work,
always include this comparison rather than only naming the two checkout types.
Never require a linked worktree for setup.

In Pi, use `development_system_worktree_list` and
`development_system_worktree_create` instead of improvising primary-checkout
mutation. After selecting or creating a worktree, ask the user to run the
returned `/development-system-worktree-switch <branch>` command in the local
TUI. That command preserves the active conversation and asks Pi to rebuild its
cwd-bound runtime in the registered worktree without restarting the process.
The model cannot invoke this user-confirmed command-context operation silently.
Headless callers use the returned relaunch command. Plain `cd` or `git -C` does
not change Pi's authoritative session checkout.
