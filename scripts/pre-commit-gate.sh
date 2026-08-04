#!/usr/bin/env bash
set -euo pipefail

if [ "${AI_PLUGINS_PRE_COMMIT_GATE_RUNNING:-}" = "1" ]; then
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if ! git diff --quiet || ! git diff --cached --quiet; then
  if ! git diff --quiet && ! git diff --cached --quiet; then
    printf 'pre-commit.gate_partial_staging_unsupported=true\n' >&2
    exit 1
  fi
fi

export AI_PLUGINS_PRE_COMMIT_GATE_RUNNING=1
exec nix develop -c just pre-commit
