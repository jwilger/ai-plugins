#!/usr/bin/env bash
# Allocate stable, non-colliding host ports for a git worktree.
set -euo pipefail

worktree_arg="${1:?usage: worktree-ports.sh <worktree-path>}"
worktree="$(cd "$worktree_arg" && pwd -P)"
base_http="${WORKTREE_PORT_BASE_HTTP:-4100}"
base_pg="${WORKTREE_PORT_BASE_PG:-5500}"
stride="${WORKTREE_PORT_STRIDE:-10}"

common_dir="$(cd "$(git -C "$worktree" rev-parse --git-common-dir)" && pwd -P)"
registry="$common_dir/worktree-ports.tsv"
lock="$registry.lock"

exec 9>"$lock"
flock 9

touch "$registry"

slot="$(awk -F'\t' -v w="$worktree" '$2 == w { print $1; exit }' "$registry")"
if [ -z "$slot" ]; then
  slot=0
  while awk -F'\t' -v s="$slot" '$1 == s { found = 1 } END { exit !found }' "$registry"; do
    slot=$((slot + 1))
  done
  printf '%s\t%s\n' "$slot" "$worktree" >>"$registry"
fi

printf 'PORT=%s\nPG_PORT=%s\n' "$((base_http + slot * stride))" "$((base_pg + slot * stride))"
