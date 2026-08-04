#!/usr/bin/env bash
# Block commits and pushes from the main checkout. Linked worktrees are allowed.
set -euo pipefail

[ "${AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN:-}" != worktree-guard ] || exit 0

git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"

if [ "$git_dir" = "$common_dir" ]; then
  echo "worktrees: commits and pushes from the main checkout are blocked; use a linked worktree." >&2
  exit 1
fi
