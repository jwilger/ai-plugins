#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
config="evals/promptfoo/agentic-systems-engineering.yaml"
out_dir="$root/evals/out"
generated_dir="$out_dir/generated"
runtime_options_file="$generated_dir/runtime-options.json"
runtime_loader_file="$generated_dir/load-harness-cases.runtime.cjs"
max_concurrency="${PROMPTFOO_MAX_CONCURRENCY:-1}"
suite="behavior"
dry_run=0
generated_config=0
promptfoo_bin="${PROMPTFOO_BIN:-$root/node_modules/.bin/promptfoo}"

usage() {
  cat <<'USAGE'
Usage: scripts/evals/run.sh [--suite behavior|canary] [config]

Runs provider-backed promptfoo evals through Claude Code and Codex.
Each provider loads the relevant marketplace surface for its harness.

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
  EVAL_PROVIDER_FILTER         (filters tested providers by variant id or provider id;
                                semantic grading still uses CODEX_GRADER_MODEL)
  PROMPTFOO_MAX_CONCURRENCY    (default: 1)

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

codex_marketplace_plugins_csv() {
  local marketplace="$root/.agents/plugins/marketplace.json"
  local plugins
  plugins="$(
    jq -er '
      if (.plugins | type) != "array" then
        empty
      else
        [.plugins[].name | select(type == "string" and length > 0)] as $names
        | if ($names | length) == 0 then empty else ($names | join(",")) end
      end
    ' "$marketplace"
  )" || {
    echo "Codex marketplace has no plugins: $marketplace" >&2
    return 2
  }
  printf '%s\n' "$plugins"
}

write_runtime_options() {
  mkdir -p "$generated_dir"
  node - "$runtime_options_file" <<'NODE'
const fs = require('fs');
const file = process.argv[2];
const options = {};
if (process.env.EVAL_CASE_FILTER) {
  options.caseFilter = process.env.EVAL_CASE_FILTER;
}
if (process.env.EVAL_SAMPLES) {
  options.samples = process.env.EVAL_SAMPLES;
}
fs.writeFileSync(file, JSON.stringify(options));
NODE
}

write_runtime_loader() {
  mkdir -p "$generated_dir"
  node - "$runtime_loader_file" "$runtime_options_file" "$root/evals/promptfoo/load-harness-cases.cjs" <<'NODE'
const fs = require('fs');
const loaderFile = process.argv[2];
const optionsFile = process.argv[3];
const baseLoader = process.argv[4];
const source = `process.env.EVAL_RUNTIME_OPTIONS_FILE = ${JSON.stringify(optionsFile)};\nmodule.exports = require(${JSON.stringify(baseLoader)});\n`;
fs.writeFileSync(loaderFile, source);
NODE
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

if [ -n "${EVAL_CASE_FILTER:-}" ]; then
  cmd+=(--filter-pattern "$EVAL_CASE_FILTER")
fi

if [ "$dry_run" -eq 1 ]; then
  dry_full_home="${CODEX_EVAL_HOME_FULL_MARKETPLACE:-${CODEX_EVAL_HOME:-$root/.dependencies/evals/codex-home-full-marketplace}}"
  dry_no_plugins_home="${CODEX_EVAL_HOME_NO_PLUGINS:-$root/.dependencies/evals/codex-home-no-plugins}"
  dry_targeted_home="${CODEX_EVAL_HOME_TARGETED_PLUGINS:-$root/.dependencies/evals/codex-home-targeted-plugins}"
  targeted_plugins="${EVAL_TARGETED_PLUGINS:-$(codex_marketplace_plugins_csv)}"
  printf '%q ' "$root/scripts/evals/ensure-node-deps.sh"
  printf '\n'
  if [ "$generated_config" -eq 1 ]; then
    printf '%q ' node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$config"
    printf '\n'
    printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$dry_full_home" --plugin-mode full-marketplace
    printf '\n'
    printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$dry_no_plugins_home" --plugin-mode no-plugins
    printf '\n'
    printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$dry_targeted_home" --plugin-mode targeted-plugins --plugins "$targeted_plugins"
    printf '\n'
  fi
  printf '%q ' "${cmd[@]}"
  printf '\n'
  exit 0
fi

cd "$root"
mkdir -p "$out_dir" "$root/.dependencies/evals/agent-workspace"
rm -f "$out_dir/results.json" "$out_dir/report.html" "$out_dir/results.junit.xml"
"$root/scripts/evals/ensure-node-deps.sh"
if [ "$generated_config" -eq 1 ]; then
  node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$config" >/dev/null
fi

export PROMPTFOO_DISABLE_TELEMETRY="${PROMPTFOO_DISABLE_TELEMETRY:-1}"
export PROMPTFOO_CONFIG_DIR="${PROMPTFOO_CONFIG_DIR:-$root/.dependencies/promptfoo}"
export PROMPTFOO_CACHE_PATH="${PROMPTFOO_CACHE_PATH:-$root/.dependencies/promptfoo-cache}"
export PROMPTFOO_CACHE_TTL="${PROMPTFOO_CACHE_TTL:-86400}"
export CODEX_EVAL_HOME="${CODEX_EVAL_HOME:-$root/.dependencies/evals/codex-home-full-marketplace}"
export CODEX_EVAL_HOME_FULL_MARKETPLACE="${CODEX_EVAL_HOME_FULL_MARKETPLACE:-$CODEX_EVAL_HOME}"
export CODEX_EVAL_HOME_NO_PLUGINS="${CODEX_EVAL_HOME_NO_PLUGINS:-$root/.dependencies/evals/codex-home-no-plugins}"
export CODEX_EVAL_HOME_TARGETED_PLUGINS="${CODEX_EVAL_HOME_TARGETED_PLUGINS:-$root/.dependencies/evals/codex-home-targeted-plugins}"
mkdir -p "$PROMPTFOO_CONFIG_DIR"

if [ "$generated_config" -eq 1 ]; then
  write_runtime_options
  write_runtime_loader
  node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME_FULL_MARKETPLACE" --plugin-mode full-marketplace >/dev/null
  node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME_NO_PLUGINS" --plugin-mode no-plugins >/dev/null
  targeted_plugins="${EVAL_TARGETED_PLUGINS:-$(codex_marketplace_plugins_csv)}"
  node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME_TARGETED_PLUGINS" --plugin-mode targeted-plugins --plugins "$targeted_plugins" >/dev/null
fi

set +e
"${cmd[@]}"
promptfoo_status="$?"
set -e

if [ "$promptfoo_status" -ne 0 ]; then
  if [ ! -s "$out_dir/results.json" ]; then
    exit "$promptfoo_status"
  fi
  node "$root/scripts/evals/check-thresholds.mjs" "$out_dir/results.json"
  exit "$?"
fi

if [ -s "$out_dir/results.json" ]; then
  node "$root/scripts/evals/check-thresholds.mjs" "$out_dir/results.json"
fi
