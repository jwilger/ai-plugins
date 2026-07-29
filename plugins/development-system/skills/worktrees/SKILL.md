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
instead of improvising primary-checkout mutation. Pi's host cwd remains the
coordination checkout. These tools persist a session-level logical workspace,
and the extension routes every built-in shell call plus relative
read/write/edit/grep/find/ls path into it independently. This works in TUI and
headless modes without a relaunch or private slash-command handoff.

Before mutation, verify the logical target with Git top-level, branch, and
status inspection. The primary checkout permits ordinary Git inspection,
exploration, tests, and builds. Keep tracked writes, Git index/history mutation,
and commits in linked logical workspaces. Read workflow policy from the
canonical primary checkout so linked worktrees inherit
`.development-system.toml` even when it is absent locally. Absolute paths must
remain inside the logical workspace and protected metadata/secret rules still
apply. Shell routing establishes the starting directory and guards common
mutations; it is not a hostile-process sandbox.

After verified delivery is complete and the logical linked worktree is clean,
call `development_system_worktree_finish`. It first persists primary logical
routing, runs repository teardown when available, removes the worktree, and
preserves the branch. Never force cleanup of dirty, detached, identity-changed,
or valuable ignored-state worktrees. A legacy Pi process launched inside the
worktree it would remove must start from primary once before finish can safely
proceed. Generated caches are inspected through a bounded exclusion-based scan
so their size cannot overflow child-process output.
