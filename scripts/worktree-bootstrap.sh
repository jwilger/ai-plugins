#!/usr/bin/env bash
# Bootstrap a linked worktree for this repository.
set -euo pipefail

[ "${AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN:-}" != worktree-bootstrap ] || exit 0

git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"

# Shared hooks run for both the main checkout and linked worktrees.
[ "$git_dir" != "$common_dir" ] || exit 0

marker="$git_dir/.ai-plugins-worktree-bootstrapped"
[ -f "$marker" ] && exit 0

worktree="$(git rev-parse --show-toplevel)"
main="$(cd "$common_dir/.." && pwd -P)"

if [ -d "$main/.dependencies" ] && [ ! -e "$worktree/.dependencies" ]; then
  mkdir -p "$worktree/.dependencies"
  for cache_entry in \
    "$main/.dependencies"/* \
    "$main/.dependencies"/.[!.]* \
    "$main/.dependencies"/..?*; do
    [ -e "$cache_entry" ] || [ -L "$cache_entry" ] || continue
    [ "${cache_entry##*/}" != evals ] || continue
    cp -a "$cache_entry" "$worktree/.dependencies/"
  done
fi

if [ -d "$main/.direnv" ] && [ ! -e "$worktree/.direnv" ]; then
  mkdir -p "$worktree/.direnv"
  cp -a "$main/.direnv/." "$worktree/.direnv/"
fi

if [ ! -e "$worktree/.envrc" ]; then
  printf 'use flake\n' >"$worktree/.envrc"
fi

{
  "$worktree/scripts/worktree-ports.sh" "$worktree"
  printf 'COMPOSE_PROJECT_NAME=%s\n' "$(basename "$worktree" | tr -c '[:alnum:]' _)"
  printf 'AI_PLUGINS_MAIN_CHECKOUT=%s\n' "$main"
} >"$worktree/.env.worktree"

touch "$marker"
