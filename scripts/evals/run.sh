#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
config="evals/promptfoo/agentic-systems-engineering.yaml"
out_dir="$root/evals/out"
generated_dir="$out_dir/generated"
max_concurrency="${PROMPTFOO_MAX_CONCURRENCY:-2}"
suite="behavior"
dry_run=0
generated_config=0
promptfoo_bin="${PROMPTFOO_BIN:-$root/node_modules/.bin/promptfoo}"

usage() {
  cat <<'USAGE'
Usage: scripts/evals/run.sh [--suite behavior|canary] [config]

Runs provider-backed promptfoo evals through Claude Code and Codex.
Full repository plugin marketplace is loaded for every scenario.

Default harness posture:
  Claude Code: provider=anthropic:claude-agent-sdk, model=sonnet, skills=all
  Codex:       provider=openai:codex-sdk, model=gpt-5.5, model_reasoning_effort=medium

Environment overrides:
  CLAUDE_EVAL_MODEL
  CODEX_EVAL_MODEL
  CODEX_EVAL_REASONING_EFFORT
  CODEX_GRADER_MODEL            (default: gpt-5.5)
  CODEX_GRADER_REASONING_EFFORT (default: medium)
  EVAL_SAMPLES
  EVAL_CASE_FILTER
  PROMPTFOO_MAX_CONCURRENCY    (default: 2)

Prompt response caching and hosted sharing are disabled for behavior evidence.
Pinned eval packages are managed by package.json and package-lock.json:
promptfoo, @openai/codex-sdk, and @anthropic-ai/claude-agent-sdk.

Requires working Claude Code and Codex model authentication.

Writes repo-owned artifacts:
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
    --suite)
      suite="$2"
      shift 2
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      case "$1" in
        /*) config="$1" ;;
        *) config="$(pwd)/$1" ;;
      esac
      shift
      ;;
  esac
done

case "$suite" in
  behavior | canary) ;;
  *)
    echo "unknown suite: $suite" >&2
    usage >&2
    exit 2
    ;;
esac

if [ "$config" = "evals/promptfoo/agentic-systems-engineering.yaml" ]; then
  config="$generated_dir/agentic-systems-engineering.${suite}.yaml"
  generated_config=1
fi

cmd=(
  "$promptfoo_bin"
  eval
  -c
  "$config"
  --max-concurrency
  "$max_concurrency"
  --no-cache
  --no-share
  -o
  "$out_dir/results.json"
  -o
  "$out_dir/report.html"
  -o
  "$out_dir/results.junit.xml"
)

if [ "$dry_run" -eq 1 ]; then
  printf '%q ' "$root/scripts/evals/ensure-node-deps.sh"
  printf '\n'
  if [ "$generated_config" -eq 1 ]; then
    printf '%q ' node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$config"
    printf '\n'
    printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$root/.dependencies/evals/codex-home"
    printf '\n'
  fi
  printf '%q ' "${cmd[@]}"
  printf '\n'
  exit 0
fi

cd "$root"
mkdir -p "$out_dir"
"$root/scripts/evals/ensure-node-deps.sh"
if [ "$generated_config" -eq 1 ]; then
  node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$config" >/dev/null
fi

export PROMPTFOO_DISABLE_TELEMETRY="${PROMPTFOO_DISABLE_TELEMETRY:-1}"
export PROMPTFOO_CACHE_PATH="${PROMPTFOO_CACHE_PATH:-$root/.dependencies/promptfoo-cache}"
export PROMPTFOO_CACHE_TTL="${PROMPTFOO_CACHE_TTL:-86400}"
export CODEX_EVAL_HOME="${CODEX_EVAL_HOME:-$root/.dependencies/evals/codex-home}"

if [ "$generated_config" -eq 1 ]; then
  node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME" >/dev/null
fi

"${cmd[@]}"
