# Coordination-checkout routing

Use this workflow only after repository-local instructions explicitly reserve
the primary checkout for coordination.

When answering a checkout-state or remediation question, give a self-contained
sequence even if the prompt supplies observations. Name the advertising policy,
both checkout-identity commands, the fetch before classification, the effective
worktree comparison against fetched upstream, the resulting state, and the exact
remediation or no-op.

## Identify and refresh

Resolve both paths; do not infer checkout identity from branch or directory
names:

```shell
git rev-parse --path-format=absolute --git-dir
git rev-parse --path-format=absolute --git-common-dir
```

Equal resolved paths identify the primary checkout; different paths identify a
linked worktree. Do not create a nested worktree when already linked. Inspect a
policy-reserved primary checkout without changing it. Resolve its configured
upstream, fetch that remote, then read status:

```shell
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}'
git fetch --prune <upstream-remote>
git status --porcelain=v1 --untracked-files=all
```

Derive `<upstream-remote>` from the configured upstream; do not assume `origin`.
No clean, dirty, or upstream-equivalent label is valid until fetch completes.

## Compare without touching the real index

Use `git merge-base --is-ancestor HEAD <fetched-upstream-ref>` to prove the
primary checkout is merely behind. Compare its effective worktree to the fetched
tree through a disposable alternate index:

```shell
comparison_dir=$(mktemp -d)
comparison_index="$comparison_dir/index"
trap 'rm -f -- "$comparison_index"; rmdir -- "$comparison_dir"' EXIT
GIT_INDEX_FILE="$comparison_index" git read-tree <fetched-upstream-ref>
GIT_INDEX_FILE="$comparison_index" git -c core.filemode=true update-index --refresh
GIT_INDEX_FILE="$comparison_index" git -c core.filemode=true diff-files --quiet --
GIT_INDEX_FILE="$comparison_index" git ls-files --others --exclude-standard -z
```

The refresh populates disposable stat data. Exit 1 because paths need update is
a comparison result to inspect with `diff-files`; a fatal error aborts
classification. `diff-files` must be empty after refresh, and the NUL-delimited
`ls-files` result must contain no paths. Never split filenames on whitespace.

The alternate upstream index handles tracked files, deletions, symlinks, type
changes, and paths added upstream but untracked relative to stale local `HEAD`.
Forcing `core.filemode=true` catches executable-bit differences even when repo
configuration ignores them. Never replace, refresh, or mutate the real index.
Do not require `git diff --cached <fetched-upstream-ref>` to be empty: an
untouched real index may correctly still match stale `HEAD`.

## Classify and route

- **Clean:** Status is empty. Create the feature branch and linked worktree from
  fetched upstream.
- **Genuinely locally dirty:** Effective content, mode, type, deletion, or an
  extra path differs from fetched upstream. Preserve every path and the real
  index exactly. Do not stage, stash, reset, clean, revert, rewrite, or include
  existing work in the feature. Create the linked worktree directly from fetched
  upstream so the coordination checkout stays untouched.
- **Upstream-equivalent dirty:** Status is nonempty relative to stale `HEAD`,
  `HEAD` is an ancestor of fetched upstream, and the alternate-index comparison
  is empty. Preserve the real index and every path. Treat the apparent changes
  as a no-op: do not rewrite, stage, stash, reset, clean, revert, or commit them.
  If the requested change already exists upstream, report no work needed;
  otherwise create the feature worktree from fetched upstream.

After confirming `.worktrees/` is ignored, use:

```shell
git worktree add .worktrees/<branch-name> -b <branch-name> <fetched-upstream-ref>
```

Run setup, baseline checks, and all feature edits inside that linked worktree.
Edit the coordination checkout only when current user direction explicitly asks
for that checkout itself to change and repository policy permits that named
exception. Otherwise stop before editing if no safe linked-worktree route exists.
