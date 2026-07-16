#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
benchmark_dir="$root/evals/benchmarks/downstream-code-quality"
dry_run=0
case_id=""

usage() {
  printf '%s\n' \
    'Usage: scripts/evals/run-code-quality-benchmark.sh --dry-run --case CASE_ID' \
    '' \
    'Plans a writable downstream Codex benchmark comparison.'
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --case)
      [ "$#" -ge 2 ] || {
        echo '--case requires a value' >&2
        exit 2
      }
      case_id="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[ "$case_id" = 'rust-cli-feature' ] || {
  printf 'unknown or missing benchmark case: %s\n' "$case_id" >&2
  exit 2
}
[ "$dry_run" -eq 1 ] || {
  echo 'live code-quality benchmark execution is not available yet' >&2
  exit 2
}

samples="${CODE_QUALITY_SAMPLES:-3}"
[[ "$samples" =~ ^([1-9]|10)$ ]] || {
  printf 'CODE_QUALITY_SAMPLES must be a canonical integer from 1 through 10; got %q\n' "$samples" >&2
  exit 2
}

work_root="$(realpath -m -- "${CODE_QUALITY_WORK_ROOT:-${TMPDIR:-/tmp}/ai-plugins-code-quality-${UID}-$$}")"
home_root="$(realpath -m -- "${CODE_QUALITY_HOME_ROOT:-$root/.dependencies/evals/code-quality-homes}")"
out_root="$(realpath -m -- "${CODE_QUALITY_OUT_ROOT:-$root/evals/out/downstream-code-quality}")"
targeted_plugins='advisor,development-discipline,engineering-standards'
modes=(no-plugins targeted-plugins full-marketplace)

paths_overlap() {
  local first="$1"
  local second="$2"
  [ "$first" = "$second" ] ||
    [ "$first" = / ] ||
    [ "$second" = / ] ||
    [[ "$second" == "$first/"* ]] ||
    [[ "$first" == "$second/"* ]]
}

assert_paths_do_not_overlap() {
  local first="$1"
  local second="$2"
  if paths_overlap "$first" "$second"; then
    printf 'benchmark paths overlap: %s and %s\n' "$first" "$second" >&2
    exit 2
  fi
}

assert_paths_do_not_overlap "$work_root" "$home_root"
assert_paths_do_not_overlap "$work_root" "$out_root"
assert_paths_do_not_overlap "$home_root" "$out_root"

print_command() {
  printf '%q ' "$@"
  printf '\n'
}

for sample in $(seq 1 "$samples"); do
  for mode in "${modes[@]}"; do
    workspace="$work_root/$case_id/sample-$sample/$mode"
    printf 'workspace %s\n' "$workspace"
    printf 'provider openai-codex-sdk-%s workspace %s\n' "$mode" "$workspace"
  done
done

print_command node "$root/scripts/evals/prepare-codex-home.mjs" \
  "$home_root/no-plugins" --plugin-mode no-plugins
print_command node "$root/scripts/evals/prepare-codex-home.mjs" \
  "$home_root/targeted-plugins" --plugin-mode targeted-plugins \
  --plugins "$targeted_plugins"
print_command node "$root/scripts/evals/prepare-codex-home.mjs" \
  "$home_root/full-marketplace" --plugin-mode full-marketplace

EVAL_OUT_DIR="$out_root" \
  EVAL_TIMEOUT=0 \
  "$root/scripts/evals/run.sh" --dry-run "$benchmark_dir/promptfooconfig.yaml"
print_command node "$root/scripts/evals/check-code-quality-benchmark.mjs" \
  "$out_root/results.json"
