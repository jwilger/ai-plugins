#!/usr/bin/env bash
# Reference worktree bootstrap — TAILOR to the project. Intended as a
# `post-checkout` hook that runs only inside a linked worktree, once.
set -euo pipefail

git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
# Inert in the main checkout (the hook is shared across all checkouts).
[ "$git_dir" != "$common_dir" ] || exit 0
marker="$git_dir/.worktree-bootstrapped"
[ -f "$marker" ] && exit 0

worktree="$(git rev-parse --show-toplevel)"
main="$(cd "$common_dir/.." && pwd -P)"
plugin_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

# 1. Warm caches (TAILOR the list to the stack).
for d in target _build deps node_modules .direnv; do
  [ -d "$main/$d" ] && rsync -a "$main/$d/" "$worktree/$d/" 2>/dev/null || true
done

# 2. Allocate ports + write the per-worktree env (TAILOR which vars/services).
{
  "$plugin_dir/scripts/worktree-ports.sh" "$worktree"
  printf 'COMPOSE_PROJECT_NAME=%s\n' "$(basename "$worktree" | tr -c '[:alnum:]' _)"
} >"$worktree/.env.worktree"

# 3. Start services / run setup (TAILOR), e.g.:
#   set -a; . "$worktree/.env.worktree"; set +a
#   docker compose up -d
#   <your-build-tool> setup

touch "$marker"
