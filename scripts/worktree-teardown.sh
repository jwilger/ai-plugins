#!/usr/bin/env bash
# Tear down per-worktree runtime state before `git worktree remove`.
set -euo pipefail

worktree="${1:?usage: worktree-teardown.sh <worktree-path>}"
worktree="$(cd "$worktree" && pwd -P)"
git -C "$worktree" rev-parse --is-inside-work-tree >/dev/null

env_file="$worktree/.env.worktree"
common_dir="$(cd "$(git -C "$worktree" rev-parse --git-common-dir)" && pwd -P)"
registry="$common_dir/worktree-ports.tsv"
lock="$registry.lock"

if [ -f "$env_file" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$env_file"
  set +a

  if [ -n "${COMPOSE_PROJECT_NAME:-}" ] && command -v docker >/dev/null 2>&1; then
    docker compose -p "$COMPOSE_PROJECT_NAME" down --volumes || true
  fi
fi

if [ -f "$registry" ]; then
  exec 9>"$lock"
  flock 9
  tmp="$(mktemp "$registry.tmp.XXXXXX")"
  awk -F'\t' -v w="$worktree" '$2 != w' "$registry" >"$tmp"
  mv "$tmp" "$registry"
fi
