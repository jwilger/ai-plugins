---
name: worktrees
description: Use when separate mutable tasks need concurrent isolation, the user explicitly requests a linked worktree, or worktree setup, cleanup, or diagnosis is needed.
---

# Worktrees

For bootstrap, cache, secret, namespace, readiness, or teardown details, load
the retained [worktree setup
contract](../../components/worktrees/skills/setup/SKILL.md).

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
`git rev-parse --absolute-git-dir` and
`git rev-parse --path-format=absolute --git-common-dir`. Different paths identify
an existing linked worktree. Reuse it for the same task; for a separate
concurrent task, create or reuse a sibling attached to the common repository,
never a worktree directory nested inside the current worktree.

Before `git worktree add`, explicitly run `git check-ignore -q -- <root>` for
the configured worktree root. Stop if it is not ignored; saying or assuming the
root is ignored is not verification. Then use the repository's supported
bootstrap workflow. Share only immutable or content-addressed caches; copy or
namespace writable build state. Load secrets from an untracked source instead of
copying them, and record per-worktree service, database/schema, volume, socket,
and port namespaces so teardown targets only that worktree. Cleanup is optional
and must refuse dirty, detached or unpublished, identity-mismatched, running, or
failed-teardown state rather than force-removing it.
