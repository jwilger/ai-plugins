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

In Pi, use `development_system_worktree_list`,
`development_system_worktree_create`, and `development_system_worktree_switch`
instead of improvising primary-checkout mutation. Local-TUI creation and switch
tools queue one-time command-context replacement automatically after the current
response; do not ask the user to type a slash command. Headless callers use the
returned relaunch command. Plain `cd` or `git -C` does not change Pi's
authoritative session checkout.

The primary checkout permits ordinary Git inspection, exploration, tests, and
builds. Keep tracked writes, Git index/history mutation, and commits in linked
worktrees. Read workflow policy from the canonical primary checkout so linked
worktrees inherit `.development-system.toml` even when it is absent locally.

After verified delivery is complete and the linked worktree is clean, call
`development_system_worktree_finish`. It returns the conversation to the
primary checkout, runs repository teardown when available, removes the worktree,
and preserves the branch. Never force cleanup of dirty or identity-changed
worktrees.
