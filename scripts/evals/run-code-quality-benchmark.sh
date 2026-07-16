#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
benchmark_dir="$root/evals/benchmarks/downstream-code-quality"
contract="$benchmark_dir/benchmark.json"
dry_run=0
case_id=""

usage() {
  printf '%s\n' \
    'Usage: scripts/evals/run-code-quality-benchmark.sh --dry-run [--case CASE_ID]' \
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

[ "$dry_run" -eq 1 ] || {
  echo 'live code-quality benchmark execution is not available yet' >&2
  exit 2
}

mapfile -t all_case_ids < <(jq -er '.cases[].id' "$contract")
if [ -n "$case_id" ]; then
  case_ids=()
  for configured_case in "${all_case_ids[@]}"; do
    if [ "$configured_case" = "$case_id" ]; then
      case_ids+=("$configured_case")
    fi
  done
  [ "${#case_ids[@]}" -eq 1 ] || {
    printf 'unknown benchmark case: %s\n' "$case_id" >&2
    exit 2
  }
else
  case_ids=("${all_case_ids[@]}")
fi

samples="${CODE_QUALITY_SAMPLES:-$(jq -er '.sampleCount' "$contract")}"
[[ "$samples" =~ ^([1-9]|10)$ ]] || {
  printf 'CODE_QUALITY_SAMPLES must be a canonical integer from 1 through 10; got %q\n' "$samples" >&2
  exit 2
}

work_root="$(realpath -m -- "${CODE_QUALITY_WORK_ROOT:-${TMPDIR:-/tmp}/ai-plugins-code-quality-${UID}-$$}")"
home_root="$(realpath -m -- "${CODE_QUALITY_HOME_ROOT:-$root/.dependencies/evals/code-quality-homes}")"
out_root="$(realpath -m -- "${CODE_QUALITY_OUT_ROOT:-$root/evals/out/downstream-code-quality}")"
targeted_plugins="$(jq -er '.conditions[] | select(.id == "targeted-quality-skills") | .plugins | join(",")' "$contract")"
mapfile -t modes < <(jq -er '.conditions[].id' "$contract")

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

for configured_case in "${case_ids[@]}"; do
  for sample in $(seq 1 "$samples"); do
    for mode in "${modes[@]}"; do
      workspace="$work_root/$configured_case/sample-$sample/$mode"
      printf 'workspace %s\n' "$workspace"
      printf 'provider openai-codex-sdk-%s workspace %s\n' "$mode" "$workspace"
    done
  done
done

printf 'metric pass@%s capability\n' "$samples"
printf 'metric pass^%s reliability\n' "$samples"
echo 'claim non-promotional'
planned_turns=$((${#case_ids[@]} * samples * ${#modes[@]}))
expected_turns="$(jq -er '.diagnosticGates.expectedExecutionTurns' "$contract")"
if [ "$planned_turns" -eq "$expected_turns" ]; then
  printf 'gate complete-runs %s/%s\n' \
    "$(jq -er '.diagnosticGates.completeRuns' "$contract")" \
    "$expected_turns"
  printf 'gate operational-errors %s\n' \
    "$(jq -er '.diagnosticGates.operationalErrors' "$contract")"
  printf 'gate provenance-errors %s\n' \
    "$(jq -er '.diagnosticGates.provenanceErrors' "$contract")"
  printf 'gate safety-failures %s\n' \
    "$(jq -er '.diagnosticGates.safetyFailures' "$contract")"
else
  echo 'diagnostic gates disabled: noncanonical run'
fi

print_command node "$root/scripts/evals/prepare-codex-home.mjs" \
  "$home_root/no-skills" --plugin-mode no-plugins --no-seed-auth
print_command node "$root/scripts/evals/prepare-codex-home.mjs" \
  "$home_root/targeted-quality-skills" --plugin-mode skills-only-marketplace \
  --plugins "$targeted_plugins" --no-seed-auth
print_command node "$root/scripts/evals/prepare-codex-home.mjs" \
  "$home_root/all-marketplace-skills" --plugin-mode skills-only-marketplace \
  --no-seed-auth

printf 'execution EVAL_CASE_FILTER=%s EVAL_SAMPLES=%s\n' "$case_id" "$samples"
EVAL_OUT_DIR="$out_root" \
  EVAL_TIMEOUT=0 \
  EVAL_CASE_FILTER="$case_id" \
  EVAL_SAMPLES="$samples" \
  "$root/scripts/evals/run.sh" --dry-run "$benchmark_dir/promptfooconfig.yaml"
print_command node "$root/scripts/evals/check-code-quality-benchmark.mjs" \
  "$out_root/results.json"
