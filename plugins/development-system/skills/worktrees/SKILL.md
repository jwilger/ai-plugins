---
name: worktrees
description: Use when separate mutable tasks need concurrent isolation, the user explicitly requests a linked worktree, or worktree setup, cleanup, or diagnosis is needed.
---

# Worktrees

Worktree support is always available. Read `[worktrees]` from
`.development-system.toml` for the configured root. Ignore any legacy
`[features].worktrees` value.

Questions and read-only investigation require no worktree. One mutable task may
use its current checkout, including the primary checkout. Use a linked worktree
when the user explicitly requests one or when a separate mutable task overlaps
across sessions or agents. Unrelated existing mutable changes count as
overlapping work and must be preserved while the new task is isolated.

Before mutation, inspect Git's top level, branch, and status with read-only
commands. These checks do not alter unrelated changes and remain required when
the user says those changes must stay untouched. Do not reinterpret "leave
existing changes untouched" as a ban on repository inspection or on creating
an independent linked worktree. When checkout identity matters, compare absolute
`--git-dir` and `--git-common-dir` paths. Different paths identify an existing
linked worktree; stay there and never nest another.

Before `git worktree add`, explicitly run `git check-ignore -q -- <root>` for
the configured worktree root. Stop if it is not ignored; saying or assuming the
root is ignored is not verification. Then use the repository's supported
bootstrap workflow. Cleanup is optional and must never force-remove dirty or
valuable state.
