---
name: setup
description: Use when making a repository worktree-ready for optional concurrent development, creating an explicitly requested linked worktree, or diagnosing worktree setup.
---

# Worktree-ready setup

Linked worktrees isolate separate mutable tasks that overlap across sessions or
agents. One mutable task may stay in the current checkout, including the
primary checkout. Unrelated existing changes make new work concurrent for
isolation purposes. Explicit user direction may also select a worktree.

Before creating one, compare `git rev-parse --absolute-git-dir` with
`git rev-parse --path-format=absolute --git-common-dir`. Different paths mean the
current checkout is already linked. Reuse that checkout for the same task; for a
separate concurrent task, create or reuse a sibling attached to the same common
repository, never a worktree directory nested inside the current worktree.
Otherwise prove the configured root is ignored before creating there. Run the
repository's setup and baseline tests in the selected linked worktree before
editing, and stop if that baseline fails.

When a repository needs bootstrap, cache, secret, service, port, lifecycle, or
command-wrapper integration, read
[the worktree-ready setup reference](references/worktree-ready-setup.md) and
tailor the supplied scripts to the detected stack. Do not install a checkout
location guard.
