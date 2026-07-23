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

max_concurrency="${PROMPTFOO_MAX_CONCURRENCY:-2}"
if [[ ! "$max_concurrency" =~ ^[12]$ ]]; then
  printf 'PROMPTFOO_MAX_CONCURRENCY must be 1 or 2; got %q\n' "$max_concurrency" >&2
  exit 2
fi

skills_home="${CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE:-${CODEX_EVAL_HOME:-$root/.evals/codex-home-skills-only-marketplace}}"
no_plugins_home="${CODEX_EVAL_HOME_NO_PLUGINS:-$root/.evals/codex-home-no-plugins}"
skills_home="$(realpath -m -- "$skills_home")"
no_plugins_home="$(realpath -m -- "$no_plugins_home")"
default_workspace="${TMPDIR:-/tmp}/ai-plugins-gpt56-workspace-${UID}-$$"
workspace="${GPT56_BENCHMARK_WORKSPACE:-$default_workspace}"
workspace="$(realpath -m -- "$workspace")"
out_root="${GPT56_BENCHMARK_OUT_ROOT:-$root/evals/out/gpt-5.6-model-family}"

case "$phase" in
  execution)
    if [ "$(realpath -m "$skills_home")" = "$(realpath -m "$no_plugins_home")" ]; then
      echo "skills-only and no-plugin Codex homes must differ" >&2
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

export CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$skills_home"
export CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home"
export GPT56_BENCHMARK_WORKSPACE="$workspace"
export PROMPTFOO_MAX_CONCURRENCY="$max_concurrency"
export EVAL_OUT_DIR="${EVAL_OUT_DIR:-$out_root/$output_suffix}"

workspace_prepare=(
  node "$root/scripts/evals/prepare-gpt56-workspace.mjs" "$workspace"
  --forbid-overlap "$root"
  --forbid-overlap "$skills_home"
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

standard_plugins_csv() {
  node -e \
    'const loadCases = require(process.argv[1]); process.stdout.write(loadCases.standardPluginNames().join(","));' \
    "$benchmark_dir/cases.cjs"
}

if [ "$dry_run" -eq 1 ]; then
  "${workspace_prepare[@]}" --check >/dev/null
  print_command "${workspace_prepare[@]}"
  if [ "$phase" = "execution" ]; then
    standard_plugins="$(standard_plugins_csv)"
    print_command node "$root/scripts/evals/prepare-codex-home.mjs" "$skills_home" --plugin-mode skills-only-marketplace --plugins "$standard_plugins"
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
  standard_plugins="$(standard_plugins_csv)"
  node "$root/scripts/evals/prepare-codex-home.mjs" "$skills_home" --plugin-mode skills-only-marketplace --plugins "$standard_plugins" >/dev/null
fi
node "$root/scripts/evals/prepare-codex-home.mjs" "$no_plugins_home" --plugin-mode no-plugins >/dev/null

if [ "$phase" = "execution" ]; then
  "$root/scripts/evals/run.sh" "$config"
  node "$root/scripts/evals/check-gpt56-measurement.mjs" \
    "$EVAL_OUT_DIR/results.json" \
    --expected-measurement-config "$config"
  node "$root/scripts/evals/check-gpt56-execution-isolation.mjs" "$EVAL_OUT_DIR/results.json"
else
  runner_status=0
  checker_status=0
  "$root/scripts/evals/run.sh" "$config" || runner_status="$?"
  case "$runner_status" in
    124 | 130 | 137 | 143)
      exit "$runner_status"
      ;;
  esac
  node "$root/scripts/evals/check-gpt56-grader-calibration.mjs" "$EVAL_OUT_DIR/results.json" || checker_status="$?"
  if [ "$runner_status" -ne 0 ]; then
    exit "$runner_status"
  fi
  exit "$checker_status"
fi
