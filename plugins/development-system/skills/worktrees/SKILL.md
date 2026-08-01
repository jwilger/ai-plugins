---
name: worktrees
description: Use when setting up worktree support, isolating concurrent mutable tickets with linked worktrees, or diagnosing worktree policy.
---

# Worktrees

Read `[features].worktrees` and `[worktrees]` from
`.development-system.toml`.

When `[worktrees].mode = "concurrent-tickets"`, use a linked worktree only to
isolate mutable tickets that are active concurrently across sessions or agents.
Ordinary questions and read-only investigation require neither a ticket nor a
worktree. A single mutable ticket may use its current checkout.

For an advisory routing question, answer from this shared skill in the same
response. References may add detail, but never return only a wait, delegation,
or file-location status while trying to locate one.

An applicable repository-local `AGENTS.md` that explicitly reserves the
primary checkout for coordination is stricter than this default and overrides
it. Do not use the single-ticket allowance in that case. Before editing or
advising on feature routing, read
[references/coordination-checkout-routing.md](references/coordination-checkout-routing.md)
and follow its fetch-first, real-index-preserving procedure. In an advisory
answer for that reserved checkout, include the minimum procedure rather than
deferring to the reference: prove primary versus linked identity by comparing
absolute `git rev-parse --git-dir` and `--git-common-dir`; resolve the
configured upstream and fetch it before classifying status; compare the
effective worktree with a disposable `GIT_INDEX_FILE` loaded from the fetched
upstream tree, refreshed before `diff-files` with `core.filemode=true`, and
use NUL-delimited extra-path checks against that same index. Preserve the real
index throughout. After the fetch, an empty status is **Clean**: start the
feature worktree from the fetched tip without touching the coordination
checkout. A nonempty status is **upstream-equivalent** only when `HEAD` is
behind and the fetched-upstream comparison is empty; that too is a no-op in
the coordination checkout. When that comparison finds a mode difference or
extra path, classify it as **genuine local work** and preserve it exactly. The
disposable index is comparison-only: never repair any result in the
coordination checkout. In particular, do not run `git update-index` against
the real index, change modes, move or remove a local path, fast-forward, stage,
stash, reset, clean, revert, rewrite, or commit there. Prove the configured
`.worktrees/` root is ignored with
`git check-ignore -q -- .worktrees`, then create the feature worktree from the
fetched upstream tip. Run the repository-defined setup and baseline checks
inside that linked worktree before editing; name them when local instructions
make the commands available, but do not invent a command for a hypothetical
repository. A general feature request is not an exception unless the repository
policy permits the named exception.

Before concurrent mutation, compare Git's absolute `--git-dir` and
`--git-common-dir`. Equality identifies the primary checkout; inequality
identifies a linked worktree. Keep the current checkout for one mutable ticket;
create or switch to linked worktrees only when separate mutable tickets need
isolation. Never create a nested worktree.

For concurrent mutable tickets, use the repository's supported linked-worktree
workflow. Before creating a new worktree, verify the configured repository-local
worktree root is ignored. Keep each session or agent on its selected checkout
and do not create a nested worktree.

Before mutation, verify the target with Git top-level, branch, and status
inspection. The current checkout permits a single ticket's tracked writes, Git
index/history mutation, commits, tests, and builds. Read workflow policy from
the canonical primary checkout when a linked worktree is in use. Absolute paths
must remain inside the selected workspace and protected metadata/secret rules
still apply. Shell routing establishes the starting directory and guards common
mutations; it is not a hostile-process sandbox.

After verified delivery, cleanup is optional housekeeping rather than a delivery
condition. If a linked worktree was created solely for concurrent-ticket
isolation and no session still needs it, run repository teardown and then remove
the clean worktree while preserving its branch. Never force cleanup of dirty,
detached, identity-changed, or valuable ignored-state worktrees.
