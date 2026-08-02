#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
benchmark_dir="$root/evals/benchmarks/gpt-5.6-model-family"
provider_eval_lock_file="$root/.evals/provider-eval.lock"
if git_common_dir="$(git -C "$root" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
  git_common_dir="$(cd "$git_common_dir" && pwd -P)"
  if [ "$(basename "$git_common_dir")" != ".git" ]; then
    echo "provider eval locking requires a non-bare coordination checkout" >&2
    exit 2
  fi
  coordination_checkout="$(cd "$git_common_dir/.." && pwd -P)"
  provider_eval_lock_file="$coordination_checkout/.evals/provider-eval.lock"
fi
phase="execution"
dry_run=0

usage() {
  cat <<'USAGE'
Usage: scripts/evals/run-gpt56-benchmark.sh [--dry-run] [--phase execution|grader-calibration]

Runs the focused GPT-5.6 execution benchmark or one frozen-answer grader
calibration through the signal-aware canonical eval runner.

Execution compares Sol, Terra, and Luna at medium effort over two standard and
two advisor-like cases. Grader calibration compares all three models at high
effort against eight frozen human-labelled answers in one run, including two
hostile tool-use prompt-injection cases.

This benchmark fixes global target-call concurrency at two as a measurement
input so latency and resource evidence remain comparable. That benchmark-only
constraint is explicit and does not replace the repository provider-eval
default of eight.

GPT56_BENCHMARK_SAMPLES controls execution repetitions (default 1; supported range 1-10).
4 cases x 3 execution providers x 1 grader per output means 24 model turns per sample
(12 execution turns plus 12 grading turns).

Options:
  --phase PHASE        execution (default) or grader-calibration
  --dry-run            print preparation and Promptfoo commands only
  --help               show this help
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --phase)
      [ "$#" -ge 2 ] || {
        echo "--phase requires a value" >&2
        exit 2
      }
      phase="$2"
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
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

samples="${GPT56_BENCHMARK_SAMPLES-1}"
if [[ ! "$samples" =~ ^([1-9]|10)$ ]]; then
  printf 'GPT56_BENCHMARK_SAMPLES must be a canonical integer from 1 through 10; got %q\n' "$samples" >&2
  exit 2
fi
export GPT56_BENCHMARK_SAMPLES="$samples"

benchmark_max_concurrency=2
if [ -n "${PROMPTFOO_MAX_CONCURRENCY+x}" ] && [ "$PROMPTFOO_MAX_CONCURRENCY" != "$benchmark_max_concurrency" ]; then
  printf 'GPT-5.6 benchmark fixes PROMPTFOO_MAX_CONCURRENCY at 2 as an explicit measurement constraint; the repository default of 8 does not apply to this benchmark; got %q\n' "$PROMPTFOO_MAX_CONCURRENCY" >&2
  exit 2
fi
max_concurrency="$benchmark_max_concurrency"

development_system_home="${CODEX_EVAL_HOME_DEVELOPMENT_SYSTEM:-${CODEX_EVAL_HOME:-$root/.evals/codex-home-development-system}}"
no_plugins_home="${CODEX_EVAL_HOME_NO_PLUGINS:-$root/.evals/codex-home-no-plugins}"
development_system_home="$(realpath -m -- "$development_system_home")"
no_plugins_home="$(realpath -m -- "$no_plugins_home")"
default_workspace="${TMPDIR:-/tmp}/ai-plugins-gpt56-workspace-${UID}-$$"
workspace="${GPT56_BENCHMARK_WORKSPACE:-$default_workspace}"
workspace="$(realpath -m -- "$workspace")"
out_root="${GPT56_BENCHMARK_OUT_ROOT:-$root/evals/out/gpt-5.6-model-family}"

case "$phase" in
  execution)
    if [ "$(realpath -m "$development_system_home")" = "$(realpath -m "$no_plugins_home")" ]; then
      echo "development-system and no-plugin Codex homes must differ" >&2
      exit 2
    fi
    config="$benchmark_dir/promptfooconfig.yaml"
    output_suffix="execution"
    ;;
  grader-calibration)
    config="$benchmark_dir/grader-promptfooconfig.yaml"
    output_suffix="grader-calibration"
    ;;
  *)
    echo "unknown benchmark phase: $phase" >&2
    exit 2
    ;;
esac

export CODEX_EVAL_HOME_DEVELOPMENT_SYSTEM="$development_system_home"
export CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home"
export GPT56_BENCHMARK_CODEX_HOME_DEVELOPMENT_SYSTEM="$development_system_home"
export GPT56_BENCHMARK_CODEX_HOME_NO_PLUGINS="$no_plugins_home"
export GPT56_BENCHMARK_WORKSPACE="$workspace"
export PROMPTFOO_MAX_CONCURRENCY="$max_concurrency"
export EVAL_OUT_DIR="${EVAL_OUT_DIR:-$out_root/$output_suffix}"

run_canonical_eval() {
  local marker_root="$workspace/.canonical-runner-context-markers"
  env \
    CODEX_EVAL_HOME="$marker_root/default" \
    CODEX_EVAL_HOME_DEVELOPMENT_SYSTEM="$marker_root/development-system" \
    CODEX_EVAL_HOME_NO_PLUGINS="$marker_root/no-plugins" \
    CODEX_EVAL_HOME_GRADER="$marker_root/grader" \
    "$root/scripts/evals/run.sh" "$config"
}

verify_artifact_scan_receipt() {
  local artifact
  local -a artifacts=()
  for artifact in \
    "$EVAL_OUT_DIR/results.json" \
    "$EVAL_OUT_DIR/report.html" \
    "$EVAL_OUT_DIR/results.junit.xml"; do
    [ ! -f "$artifact" ] || artifacts+=("$artifact")
  done
  [ "${#artifacts[@]}" -gt 0 ] || return 0
  node "$root/scripts/evals/artifact-scan-receipt.mjs" verify \
    "$EVAL_OUT_DIR/artifact-scan-receipt.json" "${artifacts[@]}"
}

workspace_prepare=(
  node "$root/scripts/evals/prepare-gpt56-workspace.mjs" "$workspace"
  --forbid-overlap "$root"
  --forbid-overlap "$development_system_home"
  --forbid-overlap "$no_plugins_home"
  --forbid-overlap "$out_root"
  --forbid-overlap "$EVAL_OUT_DIR"
)
if [ -n "${CODEX_EVAL_AUTH_HOME:-}" ]; then
  workspace_prepare+=(--forbid-overlap "$CODEX_EVAL_AUTH_HOME")
elif [ -n "${CODEX_HOME:-}" ]; then
  workspace_prepare+=(--forbid-overlap "$CODEX_HOME")
fi
if [ -n "${HOME:-}" ]; then
  workspace_prepare+=(--forbid-overlap "$HOME/.codex")
fi

print_command() {
  printf '%q ' "$@"
  printf '\n'
}

if [ "$dry_run" -eq 1 ]; then
  "${workspace_prepare[@]}" --check >/dev/null
  print_command "${workspace_prepare[@]}"
  if [ "$phase" = "execution" ]; then
    print_command node "$root/scripts/evals/prepare-codex-home.mjs" "$development_system_home" --plugin-mode development-system --install-via-cli
  fi
  print_command node "$root/scripts/evals/prepare-codex-home.mjs" "$no_plugins_home" --plugin-mode no-plugins
  "$root/scripts/evals/run.sh" --dry-run "$config"
  if [ "$phase" = "execution" ]; then
    print_command node "$root/scripts/evals/check-gpt56-measurement.mjs" "$EVAL_OUT_DIR/results.json" --expected-measurement-config "$config"
    print_command node "$root/scripts/evals/check-gpt56-execution-isolation.mjs" "$EVAL_OUT_DIR/results.json"
  else
    print_command node "$root/scripts/evals/check-gpt56-grader-calibration.mjs" "$EVAL_OUT_DIR/results.json"
  fi
  exit 0
fi

lock_file="$provider_eval_lock_file"
mkdir -p "$(dirname "$lock_file")"
exec 9>>"$lock_file"
if ! flock --nonblock 9; then
  echo "provider-backed eval already active; lock is held: $lock_file" >&2
  exit 75
fi
export AI_PLUGINS_EVAL_LOCK_HELD=1
export AI_PLUGINS_EVAL_LOCK_PATH="$lock_file"
export AI_PLUGINS_EVAL_LOCK_FD=9

"${workspace_prepare[@]}" >/dev/null
if [ "$phase" = "execution" ]; then
  node "$root/scripts/evals/prepare-codex-home.mjs" "$development_system_home" --plugin-mode development-system --install-via-cli >/dev/null
fi
node "$root/scripts/evals/prepare-codex-home.mjs" "$no_plugins_home" --plugin-mode no-plugins >/dev/null

if [ "$phase" = "execution" ]; then
  run_canonical_eval
  verify_artifact_scan_receipt
  node "$root/scripts/evals/check-gpt56-measurement.mjs" \
    "$EVAL_OUT_DIR/results.json" \
    --expected-measurement-config "$config"
  node "$root/scripts/evals/check-gpt56-execution-isolation.mjs" "$EVAL_OUT_DIR/results.json"
else
  runner_status=0
  checker_status=0
  run_canonical_eval || runner_status="$?"
  case "$runner_status" in
    124 | 130 | 137 | 143)
      exit "$runner_status"
      ;;
  esac
  verify_artifact_scan_receipt
  node "$root/scripts/evals/check-gpt56-grader-calibration.mjs" "$EVAL_OUT_DIR/results.json" || checker_status="$?"
  if [ "$runner_status" -ne 0 ]; then
    exit "$runner_status"
  fi
  exit "$checker_status"
fi
