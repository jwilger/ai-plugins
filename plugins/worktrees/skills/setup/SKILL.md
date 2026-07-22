---
name: setup
description: Use when making a repository worktree-ready for parallel development — per-worktree isolation (ports, containers, caches, secrets), lifecycle hooks, and the main-checkout enforcement guard.
---

# Worktree-ready setup

Make a repository support isolated, parallel worktree development. This is
**goal-driven**: realize the isolation goals below for _this_ project's stack —
do not copy a fixed bootstrap. The reference scripts in this plugin are starting
points to tailor, not drop-in solutions.

## Isolation goals (realize the ones that apply; skip the rest)

1. **Filesystem** — each worktree is its own checkout (inherent to git
   worktrees). Use the ignored repo-local default checkout root
   `./.worktrees/` unless the user confirms another location.
2. **Untracked config / secrets** — the worktree needs working config without
   _copying secrets around_. Prefer an upward `.env` search (the app or `.envrc`
   walks up to the main checkout's `.env`) plus a per-worktree override file.
3. **Warm build caches** — copy or link build artifacts so a new worktree starts
   fast (`target/`, `_build/`, `deps/`, `node_modules/`, `.direnv/`, …). Copy
   (rsync) when tools dislike symlinked caches; link when they don't.
4. **Service / container isolation** — give each worktree its own containers, DB,
   and volumes by namespacing on a per-worktree project name (e.g. Docker Compose
   `COMPOSE_PROJECT_NAME`), so parallel worktrees never clobber each other.
5. **Port isolation** — assign each worktree a non-colliding block of host ports.
   Use `scripts/worktree-ports.sh <worktree>` (slot-based, idempotent,
   configurable bases) and write the result into the worktree's env.
6. **Lifecycle** — bootstrap on worktree creation, tear down before removal. Wire
   a `post-checkout` hook (lefthook/husky/plain git) to the bootstrap, and run
   teardown (`docker compose … down --volumes`, then `git worktree remove`)
   before deleting a worktree. Generate plain shell scripts first; convenience
   command wrappers are optional and must fit the project.
7. **Enforcement** — install the main-checkout guard so commits and pushes only
   originate from worktrees (this plugin's `scripts/worktree-guard.sh`).

## Before feature edits in an existing repository

Apply this checkout-routing workflow only when repository-local instructions
explicitly reserve a primary checkout for coordination. Do not impose a
coordination-checkout policy merely because the repository uses Git worktrees or
this plugin is installed.

When answering a checkout-state or remediation question, do not skip directly
to the inferred state even when the prompt supplies the observations. Give a
self-contained sequence that names the advertising repository policy, both
checkout-identity commands, the fetch before classification, the effective
worktree content and mode comparison against the fetched upstream tree, the
resulting state, and the exact remediation or no-op.

Before editing, resolve both paths rather than inferring checkout identity from
the branch name or directory name:

```shell
git rev-parse --path-format=absolute --git-dir
git rev-parse --path-format=absolute --git-common-dir
```

Equal resolved paths identify the primary checkout. Different paths identify a
linked worktree. If already in a linked worktree, do not create a nested one.
If repository policy names the primary checkout as coordination-only, inspect
its state without changing it. Resolve its configured upstream, fetch that
remote, and only then read status and classify against the fetched upstream tip
rather than a stale local branch:

```shell
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}'
git fetch --prune <upstream-remote>
git status --porcelain=v1 --untracked-files=all
```

Derive `<upstream-remote>` from that configured upstream; do not assume it is
named `origin`. Fetch it before running or interpreting status. In an advisory
answer, keep the fetch command before the status command and state that no
clean/dirty/upstream-equivalent label is valid until the fetch completes.
Use `git merge-base --is-ancestor HEAD <fetched-upstream-ref>` to prove that a
dirty-looking primary checkout is merely behind. Compare the effective
worktree to that fetched tree through a disposable alternate index; never
replace, refresh, or otherwise mutate the checkout's real index:

```shell
comparison_dir=$(mktemp -d)
comparison_index="$comparison_dir/index"
trap 'rm -f -- "$comparison_index"; rmdir -- "$comparison_dir"' EXIT
GIT_INDEX_FILE="$comparison_index" git read-tree <fetched-upstream-ref>
GIT_INDEX_FILE="$comparison_index" git -c core.filemode=true update-index --refresh
GIT_INDEX_FILE="$comparison_index" git -c core.filemode=true diff-files --quiet --
GIT_INDEX_FILE="$comparison_index" git ls-files --others --exclude-standard -z
```

The refresh populates stat data in the disposable index; an exit status of 1
because paths need update is a comparison result to examine with `diff-files`,
while a fatal error aborts classification. `diff-files` must be empty after the
refresh, and the NUL-delimited `ls-files` result must contain no paths. Process
that result as NUL-delimited data rather than splitting filenames on whitespace.
Because the alternate index contains the fetched upstream tree, this single
comparison handles tracked files, deletions, symlinks, type changes, and paths
that are untracked only relative to stale local `HEAD` but were added upstream.
Forcing `core.filemode=true` makes Git executable-bit differences material even
when the repository configuration would normally ignore them.

Inspect the real index state separately, but do not require `git diff --cached
<fetched-upstream-ref>` to be empty: in the ordinary upstream-equivalent case,
the untouched index still matches stale local `HEAD` and therefore differs from
upstream. State the alternate-index checks explicitly in advisory answers; a
nonempty status alone cannot distinguish genuine local work from an
upstream-equivalent tree.

Distinguish these states:

- **Clean:** status is empty. Create the feature branch and linked worktree from
  the fetched upstream tip.
- **Genuinely locally dirty:** at least one effective tracked content, mode,
  type, deletion, or extra-path result from the alternate-index comparison
  differs from the fetched upstream tree. Preserve every
  existing path exactly. Do not stage, stash, reset, clean, revert, rewrite, or
  include it in the feature. Create the new linked worktree directly from the
  fetched upstream tip so the coordination checkout remains untouched.
- **Upstream-equivalent dirty:** status is nonempty relative to the stale local
  `HEAD`, `HEAD` is an ancestor of the fetched upstream, and the effective
  tracked worktree matches the fetched upstream tree. The index may still match
  stale `HEAD`; preserve it without treating that expected difference as local
  feature work. Paths that are untracked only because local `HEAD` predates
  their upstream addition are tracked by the alternate upstream index, which
  compares their blob bytes, type, and executable mode without special-casing
  filenames.
  Treat the apparent changes as a no-op: do not rewrite, stage, stash, reset,
  clean, revert, or commit them. If the requested change is already present
  upstream, report that no work is needed. Otherwise create the new feature
  worktree from the fetched upstream tip.

After confirming that `.worktrees/` is ignored, the exact remediation is:

```shell
git worktree add .worktrees/<branch-name> -b <branch-name> <fetched-upstream-ref>
```

Perform setup, baseline checks, and all feature edits inside that linked
worktree. A general request to start or continue a feature is not an exception.
Edit the coordination checkout only when current user direction explicitly asks
for that checkout itself to be changed and repository policy permits the named
exception; otherwise stop before editing if no safe linked-worktree route is
available.

## How to apply

1. Detect the stack: language, package manager, containers, dev server.
2. Decide which goals apply and how (which caches, which services, which ports).
3. Use `./.worktrees/<name>` as the default checkout path in generated examples
   and helper scripts. Ensure `.worktrees/` is ignored before relying on it.
4. Generate a project-specific bootstrap (adapt `templates/bootstrap-worktree.sh`)
   wired into the project's hook manager, and a teardown (adapt
   `templates/worktree-teardown.sh`).
5. Detect the project command surface before adding workflow shortcuts:
   `justfile`, `Makefile`, `package.json` scripts, project-specific task runners,
   or no wrapper at all. Do not assume `just`, npm, make, or any other runner is
   present across projects.
6. Present the detected command surface and confirm the selected wrapper with
   the user before editing it. If no wrapper is confirmed, document direct shell
   usage only.
7. Generate `bats` tests for the generated scripts and any confirmed wrapper.
8. Install the enforcement guard as the `pre-commit` and `pre-push` hooks.
9. Document the resulting workflow in the project's `AGENTS.md`.
