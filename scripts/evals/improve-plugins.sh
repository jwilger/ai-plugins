#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$root"
scripts/evals/check-loop-scope.sh improve-plugins
node scripts/evals/check-coverage.mjs
scripts/evals/run.sh --dry-run >/dev/null
scripts/evals/check-loop-scope.sh improve-plugins

cat <<'MSG'
Plugin improvement loop scope is valid.
Only plugin instruction assets may be edited in this loop.
MSG
