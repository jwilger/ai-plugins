#!/usr/bin/env bash
# Worktree enforcement guard.
#
# Blocks commits and pushes that originate from the MAIN checkout rather than a
# linked worktree. Install as the repository's pre-commit and pre-push hooks.
# Deterministic and self-healing: an attempt to commit in the main checkout
# fails, steering the work into a linked worktree.
set -euo pipefail

git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"

if [ "$git_dir" = "$common_dir" ]; then
  echo "worktrees: changes from the main checkout are blocked — work in a linked worktree." >&2
  exit 1
fi
