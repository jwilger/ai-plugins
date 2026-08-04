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

# Git exports repository-local variables to hooks.  They are correct for the
# commit currently being checked, but would corrupt independent Git fixtures
# created by the gate's test suites.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX GIT_COMMON_DIR

export AI_PLUGINS_PRE_COMMIT_GATE_RUNNING=1
exec nix develop -c just pre-commit
