#!/usr/bin/env bash
# Bootstrap a linked worktree for this repository.
set -euo pipefail

git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"

# Shared hooks run for both the main checkout and linked worktrees.
[ "$git_dir" != "$common_dir" ] || exit 0

marker="$git_dir/.ai-plugins-worktree-bootstrapped"
[ -f "$marker" ] && exit 0

worktree="$(git rev-parse --show-toplevel)"
main="$(cd "$common_dir/.." && pwd -P)"

for cache_dir in .dependencies .direnv; do
  if [ -d "$main/$cache_dir" ] && [ ! -e "$worktree/$cache_dir" ]; then
    mkdir -p "$worktree/$cache_dir"
    cp -a "$main/$cache_dir/." "$worktree/$cache_dir/" 2>/dev/null || true
  fi
done

if [ ! -e "$worktree/.envrc" ]; then
  printf 'use flake\n' >"$worktree/.envrc"
fi

{
  "$worktree/scripts/worktree-ports.sh" "$worktree"
  printf 'COMPOSE_PROJECT_NAME=%s\n' "$(basename "$worktree" | tr -c '[:alnum:]' _)"
  printf 'AI_PLUGINS_MAIN_CHECKOUT=%s\n' "$main"
} >"$worktree/.env.worktree"

touch "$marker"
