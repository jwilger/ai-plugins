#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
caller_cwd="$(pwd -P)"
config="evals/promptfoo/agentic-systems-engineering.yaml"
default_out_dir="$root/evals/out"
requested_out_dir="${EVAL_OUT_DIR:-$default_out_dir}"
case "$requested_out_dir" in
  /*) out_dir="$(realpath -m -- "$requested_out_dir")" ;;
  *) out_dir="$(realpath -m -- "$caller_cwd/$requested_out_dir")" ;;
esac
export EVAL_OUT_DIR="$out_dir"
generated_dir="$out_dir/generated"
runtime_options_file="$generated_dir/runtime-options.json"
runtime_loader_file="$generated_dir/load-harness-cases.runtime.cjs"
export EVAL_RUNTIME_LOADER_FILE="$runtime_loader_file"
max_concurrency="${PROMPTFOO_MAX_CONCURRENCY:-1}"
case "$max_concurrency" in
  1 | 2) ;;
  *)
    printf 'PROMPTFOO_MAX_CONCURRENCY must be 1 or 2; got %q\n' "$max_concurrency" >&2
    exit 2
    ;;
esac
eval_timeout="${EVAL_TIMEOUT:-}"
eval_timeout_full_default="${EVAL_TIMEOUT_FULL_DEFAULT:-90m}"
eval_timeout_focused_default="${EVAL_TIMEOUT_FOCUSED_DEFAULT:-20m}"
eval_timeout_kill_after="${EVAL_TIMEOUT_KILL_AFTER:-30s}"
eval_interrupt_grace="${EVAL_INTERRUPT_GRACE:-2s}"
suite="behavior"
dry_run=0
generated_config=0
promptfoo_bin="${PROMPTFOO_BIN:-$root/node_modules/.bin/promptfoo}"
eval_pid=""
eval_watchdog_pid=""
eval_launching=0
interrupted_status=0
interrupted_signal=""
provider_eval_lock_file="$root/.dependencies/evals/provider-eval.lock"
if git_common_dir="$(git -C "$root" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
  git_common_dir="$(cd "$git_common_dir" && pwd -P)"
  if [ "$(basename "$git_common_dir")" != ".git" ]; then
    echo "provider eval locking requires a non-bare coordination checkout" >&2
    exit 2
  fi
  coordination_checkout="$(cd "$git_common_dir/.." && pwd -P)"
  provider_eval_lock_file="$coordination_checkout/.dependencies/evals/provider-eval.lock"
fi

usage() {
  cat <<'USAGE'
Usage: scripts/evals/run.sh [--suite behavior|canary] [config]

Runs provider-backed promptfoo evals through Claude Code and Codex.
Each provider loads the relevant marketplace surface for its harness.

Default harness posture:
  Claude Code: provider=anthropic:claude-agent-sdk, model=sonnet, skills=all
  Codex:       provider=openai:codex-sdk, model=gpt-5.6-terra, model_reasoning_effort=medium

Environment overrides:
  CLAUDE_EVAL_MODEL
  CODEX_EVAL_MODEL
  CODEX_EVAL_REASONING_EFFORT
  CODEX_GRADER_MODEL            (default: gpt-5.6-sol)
  CODEX_GRADER_REASONING_EFFORT (default: high)
  EVAL_SAMPLES
  EVAL_CASE_FILTER
  EVAL_PROVIDER_FILTER         (filters tested providers by final label, variant id,
                                provider id, plugin mode, or substring;
                                an exact variant id selects full-marketplace only;
                                semantic grading still uses CODEX_GRADER_MODEL)
  PROMPTFOO_MAX_CONCURRENCY    (allowed: 1-2; default: 1)
  EVAL_TIMEOUT                 (default: 90m for full behavior runs, 20m otherwise;
                                set to 0 to disable)
  EVAL_TIMEOUT_FULL_DEFAULT    (default: 90m)
  EVAL_TIMEOUT_FOCUSED_DEFAULT (default: 20m)
  EVAL_TIMEOUT_KILL_AFTER      (default: 30s; force-kill grace period)
  EVAL_INTERRUPT_GRACE         (default: 2s between INT, TERM, and KILL)
  EVAL_OUT_DIR                 (default: evals/out; isolates generated config and artifacts)

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

retain_partial_outputs() {
  local reason="$1"
  local retention_parent="$out_dir/timeout-artifacts"
  local retention_dir
  local retained=0
  mkdir -p "$retention_parent"
  retention_dir="$(mktemp -d "$retention_parent/$(date -u +%Y%m%dT%H%M%SZ)-$reason.XXXXXX")"
  for artifact in "$out_dir/results.json" "$out_dir/report.html" "$out_dir/results.junit.xml"; do
    if [ -e "$artifact" ]; then
      mv "$artifact" "$retention_dir/"
      retained=1
    fi
  done
  if [ "$retained" -eq 1 ]; then
    echo "retained partial eval artifacts in $retention_dir" >&2
  else
    rmdir "$retention_dir"
  fi
}

write_eval_status() {
  local state="$1"
  local reason="$2"
  node "$root/scripts/evals/write-status.mjs" \
    --output "$out_dir/status.json" \
    --state "$state" \
    --reason "$reason" \
    --provider-credentials "${EVAL_PROVIDER_CREDENTIALS_STATUS:-unknown}" >/dev/null
}

finish_eval_interruption() {
  local status="$1"
  local message

  if [ "$status" -eq 143 ]; then
    message="promptfoo eval was terminated before completion"
  else
    message="promptfoo eval was interrupted before completion with status $status"
  fi
  echo "$message" >&2
  write_eval_status interrupted "$message"
  retain_partial_outputs "exit-$status"
  exit "$status"
}

forward_eval_signal() {
  local signal="$1"
  local status="$2"

  interrupted_status="$status"
  interrupted_signal="$signal"
  if [ -n "$eval_pid" ]; then
    kill -s "$signal" -- "-$eval_pid" 2>/dev/null ||
      kill -s "$signal" "$eval_pid" 2>/dev/null || true
    arm_eval_interrupt_watchdog
  elif [ "$eval_launching" -eq 1 ]; then
    return 0
  else
    trap - INT TERM
    finish_eval_interruption "$status"
  fi
}

arm_eval_interrupt_watchdog() {
  local group_id="$eval_pid"

  [ -z "$eval_watchdog_pid" ] || return 0
  (
    sleep "$eval_interrupt_grace"
    if kill -0 -- "-$group_id" 2>/dev/null || kill -0 "$group_id" 2>/dev/null; then
      kill -TERM -- "-$group_id" 2>/dev/null || kill -TERM "$group_id" 2>/dev/null || true
    else
      exit 0
    fi
    sleep "$eval_interrupt_grace"
    if kill -0 -- "-$group_id" 2>/dev/null || kill -0 "$group_id" 2>/dev/null; then
      kill -KILL -- "-$group_id" 2>/dev/null || kill -KILL "$group_id" 2>/dev/null || true
    fi
  ) &
  eval_watchdog_pid="$!"
}

selected_codex_provider_compositions() {
  node "$root/scripts/evals/provider-compositions.mjs" \
    "$generated_metadata_file" \
    "$1" \
    "$2" \
    "$3"
}

uses_codex_grader() {
  node - "$generated_metadata_file" <<'NODE'
const fs = require('fs');
const metadata = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
process.exit(metadata.usesCodexGrader ? 0 : 1);
NODE
}

prepare_codex_home_for_mode() {
  local mode="$1"
  local plugins="${2:-}"
  case "$mode" in
    full-marketplace)
      node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME_FULL_MARKETPLACE" --plugin-mode full-marketplace >/dev/null
      ;;
    no-plugins)
      node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME_NO_PLUGINS" --plugin-mode no-plugins >/dev/null
      ;;
    targeted-plugins)
      node "$root/scripts/evals/prepare-codex-home.mjs" "$CODEX_EVAL_HOME_TARGETED_PLUGINS" --plugin-mode targeted-plugins --plugins "$plugins" >/dev/null
      ;;
    *)
      echo "unknown Codex plugin mode in generated eval config: $mode" >&2
      return 2
      ;;
  esac
}

print_prepare_codex_home_for_mode() {
  local mode="$1"
  local plugins="${2:-}"
  case "$mode" in
    full-marketplace)
      printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$dry_full_home" --plugin-mode full-marketplace
      printf '\n'
      ;;
    no-plugins)
      printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$dry_no_plugins_home" --plugin-mode no-plugins
      printf '\n'
      ;;
    targeted-plugins)
      printf '%q ' node "$root/scripts/evals/prepare-codex-home.mjs" "$dry_targeted_home" --plugin-mode targeted-plugins --plugins "$plugins"
      printf '\n'
      ;;
    *)
      echo "unknown Codex plugin mode in generated eval config: $mode" >&2
      return 2
      ;;
  esac
}

acquire_provider_eval_lock() {
  local inherited_fd="${AI_PLUGINS_EVAL_LOCK_FD:-}"
  local inherited_path="${AI_PLUGINS_EVAL_LOCK_PATH:-}"
  local canonical_inherited_path=""

  if [ -n "$inherited_path" ]; then
    canonical_inherited_path="$(realpath -m -- "$inherited_path")"
  fi
  if [ "${AI_PLUGINS_EVAL_LOCK_HELD:-}" = "1" ] &&
    [ "$canonical_inherited_path" = "$provider_eval_lock_file" ] &&
    [[ "$inherited_fd" =~ ^[0-9]+$ ]] &&
    [ "$provider_eval_lock_file" -ef "/dev/fd/$inherited_fd" ] &&
    flock --nonblock "$inherited_fd"; then
    return 0
  fi

  mkdir -p "$(dirname "$provider_eval_lock_file")"
  exec 9>>"$provider_eval_lock_file"
  if ! flock --nonblock 9; then
    echo "provider-backed eval already active; lock is held: $provider_eval_lock_file" >&2
    exit 75
  fi
  export AI_PLUGINS_EVAL_LOCK_HELD=1
  export AI_PLUGINS_EVAL_LOCK_PATH="$provider_eval_lock_file"
  export AI_PLUGINS_EVAL_LOCK_FD=9
}

prepare_eval_output_dir() {
  local mode="${1:-prepare}"
  node - "$out_dir" "$default_out_dir" "$root" "$mode" <<'NODE'
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const outputDir = process.argv[2];
const defaultOutputDir = process.argv[3];
const repoRoot = process.argv[4];
const checkOnly = process.argv[5] === 'check';
const markerName = '.ai-plugins-eval-output';
const markerContents = 'ai-plugins eval output\n';

function realPathIfExists(entry) {
  try {
    return fs.realpathSync(entry);
  } catch {
    return path.resolve(entry);
  }
}

function isSameOrAncestor(ancestor, descendant) {
  const relative = path.relative(ancestor, descendant);
  return (
    relative === '' ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== '..' &&
      !path.isAbsolute(relative))
  );
}

try {
  const realOutputDir = realPathIfExists(outputDir);
  for (const protectedRoot of [repoRoot, os.homedir()]) {
    if (isSameOrAncestor(realOutputDir, realPathIfExists(protectedRoot))) {
      throw new Error(
        `eval output path contains protected root: ${protectedRoot}`,
      );
    }
  }

  if (!fs.existsSync(outputDir)) {
    if (checkOnly) process.exit(0);
    fs.mkdirSync(outputDir, { recursive: true });
  }
  if (!fs.statSync(outputDir).isDirectory()) {
    throw new Error(`eval output path is not a directory: ${outputDir}`);
  }

  const entries = fs.readdirSync(outputDir);
  const marker = path.join(outputDir, markerName);
  const isRepoEvalOutput = isSameOrAncestor(
    realPathIfExists(defaultOutputDir),
    realPathIfExists(outputDir),
  );
  const isOwned =
    fs.existsSync(marker) &&
    fs.readFileSync(marker, 'utf8') === markerContents;

  if (entries.length > 0 && !isRepoEvalOutput && !isOwned) {
    throw new Error(`refusing unowned eval output directory: ${outputDir}`);
  }
  if (entries.length === 0 && !checkOnly) {
    fs.writeFileSync(marker, markerContents, { mode: 0o600 });
  }
} catch (error) {
  console.error(error.message);
  process.exit(2);
}
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

generated_metadata_file="$generated_dir/agentic-systems-engineering.${suite}.metadata.json"

if [ "$config" = "evals/promptfoo/agentic-systems-engineering.yaml" ]; then
  config="$generated_dir/agentic-systems-engineering.${suite}.yaml"
  generated_config=1
fi

if [ -z "$eval_timeout" ]; then
  if [ "$generated_config" -eq 1 ] &&
    [ "$suite" = "behavior" ] &&
    [ -z "${EVAL_CASE_FILTER:-}" ] &&
    [ -z "${EVAL_PROVIDER_FILTER:-}" ] &&
    [ -z "${EVAL_SAMPLES:-}" ]; then
    eval_timeout="$eval_timeout_full_default"
  else
    eval_timeout="$eval_timeout_focused_default"
  fi
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

run_cmd=(timeout --kill-after "$eval_timeout_kill_after" "$eval_timeout" "${cmd[@]}")

if [ "$dry_run" -eq 0 ]; then
  acquire_provider_eval_lock
fi

cd "$root"

if [ "$dry_run" -eq 1 ]; then
  prepare_eval_output_dir check
  dry_full_home="${CODEX_EVAL_HOME_FULL_MARKETPLACE:-${CODEX_EVAL_HOME:-$root/.dependencies/evals/codex-home-full-marketplace}}"
  dry_no_plugins_home="${CODEX_EVAL_HOME_NO_PLUGINS:-$root/.dependencies/evals/codex-home-no-plugins}"
  dry_targeted_home="${CODEX_EVAL_HOME_TARGETED_PLUGINS:-$root/.dependencies/evals/codex-home-targeted-plugins}"
  printf '%q ' "$root/scripts/evals/ensure-node-deps.sh"
  printf '\n'
  if [ "$generated_config" -eq 1 ]; then
    generated_metadata_output_file="$generated_metadata_file"
    dry_inspection_dir="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-eval-dry-run.XXXXXX")"
    trap 'rm -rf -- "$dry_inspection_dir"' EXIT
    dry_inspection_config="$dry_inspection_dir/config.yaml"
    generated_metadata_file="$dry_inspection_dir/metadata.json"
    node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$dry_inspection_config" --metadata-output "$generated_metadata_file" >/dev/null
    codex_provider_compositions="$(selected_codex_provider_compositions \
      "$dry_full_home" \
      "$dry_no_plugins_home" \
      "$dry_targeted_home")"
    printf '%q ' node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$config" --metadata-output "$generated_metadata_output_file"
    printf '\n'
    if uses_codex_grader; then
      print_prepare_codex_home_for_mode full-marketplace
    fi
    if [ -n "$codex_provider_compositions" ]; then
      while IFS=$'\t' read -r mode provider_plugins; do
        [ "$mode" != "full-marketplace" ] || continue
        print_prepare_codex_home_for_mode "$mode" "$provider_plugins"
      done <<<"$codex_provider_compositions"
    fi
  fi
  printf '%q ' "${run_cmd[@]}"
  printf '\n'
  exit 0
fi

prepare_eval_output_dir
mkdir -p "$out_dir" "$root/.dependencies/evals/agent-workspace"
rm -f "$out_dir/results.json" "$out_dir/report.html" "$out_dir/results.junit.xml" "$out_dir/status.json"
trap 'forward_eval_signal INT 130' INT
trap 'forward_eval_signal TERM 143' TERM
"$root/scripts/evals/ensure-node-deps.sh"
if [ "$generated_config" -eq 1 ]; then
  node "$root/scripts/evals/generate-config.mjs" --suite "$suite" --output "$config" --metadata-output "$generated_metadata_file" >/dev/null
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
  codex_provider_compositions="$(selected_codex_provider_compositions \
    "$CODEX_EVAL_HOME_FULL_MARKETPLACE" \
    "$CODEX_EVAL_HOME_NO_PLUGINS" \
    "$CODEX_EVAL_HOME_TARGETED_PLUGINS")"
  write_runtime_options
  write_runtime_loader
  if uses_codex_grader; then
    prepare_codex_home_for_mode full-marketplace
  fi
  if [ -n "$codex_provider_compositions" ]; then
    while IFS=$'\t' read -r mode provider_plugins; do
      [ "$mode" != "full-marketplace" ] || continue
      prepare_codex_home_for_mode "$mode" "$provider_plugins"
    done <<<"$codex_provider_compositions"
  fi
fi

set +e
eval_launching=1
(
  trap 'exit 130' INT
  trap 'exit 143' TERM
  exec "${run_cmd[@]}"
) &
eval_pid="$!"
eval_launching=0
if [ -n "$interrupted_signal" ]; then
  kill -s "$interrupted_signal" -- "-$eval_pid" 2>/dev/null ||
    kill -s "$interrupted_signal" "$eval_pid" 2>/dev/null || true
  arm_eval_interrupt_watchdog
fi
promptfoo_status=0
while true; do
  wait "$eval_pid"
  promptfoo_status="$?"
  if ! kill -0 "$eval_pid" 2>/dev/null; then
    break
  fi
done
if [ -n "$eval_watchdog_pid" ]; then
  wait "$eval_watchdog_pid" 2>/dev/null || true
  eval_watchdog_pid=""
fi
trap - INT TERM
eval_pid=""
if [ "$interrupted_status" -ne 0 ]; then
  promptfoo_status="$interrupted_status"
fi
set -e

if [ "$promptfoo_status" -ne 0 ]; then
  if [ "$promptfoo_status" -eq 124 ] || [ "$promptfoo_status" -eq 137 ]; then
    echo "promptfoo eval timed out after EVAL_TIMEOUT=$eval_timeout" >&2
    write_eval_status timed-out "promptfoo eval timed out after EVAL_TIMEOUT=$eval_timeout"
    retain_partial_outputs "exit-$promptfoo_status"
    exit "$promptfoo_status"
  fi
  if [ "$promptfoo_status" -eq 143 ]; then
    finish_eval_interruption "$promptfoo_status"
  fi
  if [ "$promptfoo_status" -ge 128 ]; then
    finish_eval_interruption "$promptfoo_status"
  fi
  if [ ! -s "$out_dir/results.json" ]; then
    exit "$promptfoo_status"
  fi
  node "$root/scripts/evals/check-thresholds.mjs" "$out_dir/results.json"
  exit "$?"
fi

if [ -s "$out_dir/results.json" ]; then
  node "$root/scripts/evals/check-thresholds.mjs" "$out_dir/results.json"
fi
