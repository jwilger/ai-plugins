#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
config="evals/promptfoo/agentic-systems-engineering.yaml"
out_dir="$root/evals/out"
promptfoo_version="${PROMPTFOO_VERSION:-latest}"
dry_run=0

usage() {
  cat <<'USAGE'
Usage: scripts/evals/run.sh [config]

Runs local promptfoo evals and writes repo-owned artifacts:
  evals/out/results.json
  evals/out/report.html
  evals/out/results.junit.xml

Options:
  --help     Show this help.
  --dry-run  Print the promptfoo command without executing it.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --help)
      usage
      exit 0
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      config="$1"
      shift
      ;;
  esac
done

mkdir -p "$out_dir"

cmd=(
  npx
  "--yes"
  "promptfoo@${promptfoo_version}"
  eval
  -c
  "$config"
  -o
  "$out_dir/results.json"
  -o
  "$out_dir/report.html"
  -o
  "$out_dir/results.junit.xml"
)

if [ "$dry_run" -eq 1 ]; then
  printf '%q ' "${cmd[@]}"
  printf '\n'
  exit 0
fi

cd "$root"
export PROMPTFOO_DISABLE_TELEMETRY="${PROMPTFOO_DISABLE_TELEMETRY:-1}"
export PROMPTFOO_CACHE_PATH="${PROMPTFOO_CACHE_PATH:-$root/.dependencies/promptfoo-cache}"

"${cmd[@]}"
