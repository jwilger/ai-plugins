#!/usr/bin/env bash
# Allocate a non-colliding block of host ports for a git worktree.
#
#   worktree-ports.sh <worktree-abs-path>
#
# Prints shell assignments (PORT, PG_PORT) for a slot-based block, so each
# worktree can run its own services without colliding. A shared, flock'd registry
# under the repository's common git dir maps each worktree to a stable slot, so
# re-running for the same worktree is idempotent.
#
# Configure the bases/stride via SIDEQUEST_PORT_BASE_HTTP (default 4100),
# SIDEQUEST_PORT_BASE_PG (5500), SIDEQUEST_PORT_STRIDE (10).
#
# Note: `flock` is util-linux; on macOS install it or substitute an equivalent.
set -euo pipefail

worktree="${1:?usage: worktree-ports.sh <worktree-path>}"
base_http="${SIDEQUEST_PORT_BASE_HTTP:-4100}"
base_pg="${SIDEQUEST_PORT_BASE_PG:-5500}"
stride="${SIDEQUEST_PORT_STRIDE:-10}"

common_dir="$(cd "$(git -C "$worktree" rev-parse --git-common-dir)" && pwd -P)"
registry="$common_dir/sidequest-ports.tsv"
lock="$registry.lock"

exec 9>"$lock"
flock 9

touch "$registry"

# Reuse this worktree's existing slot if present, else take the lowest free one.
slot="$(awk -F'\t' -v w="$worktree" '$2 == w { print $1; exit }' "$registry")"
if [ -z "$slot" ]; then
  slot=0
  while awk -F'\t' -v s="$slot" '$1 == s { found = 1 } END { exit !found }' "$registry"; do
    slot=$((slot + 1))
  done
  printf '%s\t%s\n' "$slot" "$worktree" >>"$registry"
fi

printf 'PORT=%s\nPG_PORT=%s\n' "$((base_http + slot * stride))" "$((base_pg + slot * stride))"
