#!/usr/bin/env bash
# Reference worktree teardown — TAILOR to the project. Run BEFORE
# `git worktree remove`, while the worktree's generated env still exists.
set -euo pipefail

worktree="${1:?usage: worktree-teardown.sh <worktree-path>}"
env_file="$worktree/.env.worktree"

if [ -f "$env_file" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$env_file"
  set +a
  if [ -n "${COMPOSE_PROJECT_NAME:-}" ] && command -v docker >/dev/null 2>&1; then
    docker compose -p "$COMPOSE_PROJECT_NAME" down --volumes || true
  fi
fi
