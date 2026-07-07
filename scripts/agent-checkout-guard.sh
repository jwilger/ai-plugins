#!/usr/bin/env bash
# Guard agent work from accidentally mutating the coordination checkout.
set -euo pipefail

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  echo "agent-checkout-guard: not inside a git repository" >&2
  exit 2
fi

repo="$(git rev-parse --show-toplevel)"
git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"

if [ "$git_dir" != "$common_dir" ]; then
  exit 0
fi

branch="$(git -C "$repo" branch --show-current 2>/dev/null || true)"
[ -n "$branch" ] || branch="detached-head"

dirty_status="$(git -C "$repo" status --porcelain=v1 --untracked-files=all)"
upstream="$(git -C "$repo" rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true)"

tracked_matches_upstream() {
  [ -n "$upstream" ] || return 1
  git -C "$repo" merge-base --is-ancestor HEAD "$upstream" >/dev/null 2>&1 || return 1

  local path
  local -a pathspecs
  pathspecs=(".")
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    pathspecs+=(":(exclude)$path")
  done < <(git -C "$repo" ls-files --others --exclude-standard)

  git -C "$repo" diff --quiet "$upstream" -- "${pathspecs[@]}" || return 1
}

untracked_matches_upstream() {
  [ -n "$upstream" ] || return 1

  local path
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    git -C "$repo" cat-file -e "$upstream:$path" 2>/dev/null || return 1
    git -C "$repo" show "$upstream:$path" | cmp -s - "$repo/$path" || return 1
  done < <(git -C "$repo" ls-files --others --exclude-standard)
}

echo "agent-checkout-guard: '$repo' is the main checkout on branch '$branch'." >&2

if [ -n "$dirty_status" ] && tracked_matches_upstream && untracked_matches_upstream; then
  cat >&2 <<EOF
agent-checkout-guard: the dirty worktree matches upstream '$upstream'.
Do not continue feature work here. First clean or fast-forward the coordination
checkout intentionally, then create a linked worktree:

  git worktree add .worktrees/<branch-name> -b <branch-name>
EOF
  exit 1
fi

if [ -n "$dirty_status" ]; then
  cat >&2 <<'EOF'
agent-checkout-guard: the main checkout already has local changes.
Do not add more edits here unless the user explicitly requested main-checkout
edits. Preserve the existing state and move feature work to a linked worktree:

  git worktree add .worktrees/<branch-name> -b <branch-name>
EOF
  exit 1
fi

cat >&2 <<'EOF'
agent-checkout-guard: the main checkout is coordination-only.
Create a linked worktree for feature work:

  git worktree add .worktrees/<branch-name> -b <branch-name>
EOF
exit 1
