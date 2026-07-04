#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$root"
scripts/evals/check-loop-scope.sh improve-evals
node scripts/evals/check-coverage.mjs
scripts/evals/run.sh --dry-run >/dev/null
scripts/evals/check-loop-scope.sh improve-evals

cat <<'MSG'
Eval improvement loop scope is valid.
Only eval fixtures, harnesses, reports, tests, and CI wiring may be edited in this loop.
MSG
