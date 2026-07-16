#!/usr/bin/env bash
set -euo pipefail
umask 077

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
benchmark_dir="$root/evals/benchmarks/downstream-code-quality"
contract="$benchmark_dir/benchmark.json"
workspace_preparer="$root/scripts/evals/prepare-code-quality-workspaces.mjs"
runtime_preparer="$root/scripts/evals/prepare-code-quality-runtime.mjs"
boundary_launcher="$root/scripts/evals/code-quality-codex-boundary"
boundary_runtime="$root/scripts/evals/code-quality-codex-boundary.mjs"
codex_resolver="$root/scripts/evals/resolve-code-quality-codex.mjs"
secret_scanner="$root/scripts/evals/scan-code-quality-secrets.mjs"
result_checker="$root/scripts/evals/check-code-quality-benchmark.mjs"
secret_scan_options=(
  --secret-env CODE_QUALITY_OPENAI_API_KEY
  --secret-env OPENAI_API_KEY
  --secret-env CODEX_API_KEY
  --secret-env ANTHROPIC_API_KEY
  --secret-env AZURE_OPENAI_API_KEY
  --secret-env GITHUB_TOKEN
  --secret-env GH_TOKEN
  --secret-env AWS_SECRET_ACCESS_KEY
  --secret-env AWS_SESSION_TOKEN
  --secret-env AZURE_CLIENT_SECRET
  --secret-env GOOGLE_API_KEY
  --secret-env NPM_TOKEN
  --secret-env NODE_AUTH_TOKEN
  --secret-env PYPI_API_TOKEN
  --secret-env HUGGING_FACE_HUB_TOKEN
  --secret-env HF_TOKEN
  --secret-env SLACK_BOT_TOKEN
  --secret-env STRIPE_SECRET_KEY
  --secret-env DATABASE_URL
  --secret-env CODECOV_TOKEN
)
dry_run=0
runtime_preflight=0
case_id=""

usage() {
  printf '%s\n' \
    'Usage: scripts/evals/run-code-quality-benchmark.sh [--dry-run | --runtime-preflight] [--case CASE_ID]' \
    '' \
    'Runs the isolated nine-turn downstream Codex code-quality diagnostic.' \
    'Live execution requires CODE_QUALITY_OPENAI_API_KEY.' \
    '--runtime-preflight prints the pinned candidate PATH and Nix runtime closure.'
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
    --runtime-preflight)
      runtime_preflight=1
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

if [ "$dry_run" -eq 1 ] && [ "$runtime_preflight" -eq 1 ]; then
  echo '--dry-run and --runtime-preflight are mutually exclusive' >&2
  exit 2
fi

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
  case_id="${case_ids[0]}"
fi

samples="${CODE_QUALITY_SAMPLES:-$(jq -er '.sampleCount' "$contract")}"
[[ "$samples" =~ ^([1-9]|10)$ ]] || {
  printf 'CODE_QUALITY_SAMPLES must be a canonical integer from 1 through 10; got %q\n' "$samples" >&2
  exit 2
}

scratch_plan="${TMPDIR:-/tmp}/ai-plugins-code-quality-${UID}-$$"
work_root="$(realpath -m -- "${CODE_QUALITY_WORK_ROOT:-$scratch_plan/workspaces}")"
runtime_root="$(realpath -m -- "${CODE_QUALITY_RUNTIME_ROOT:-$scratch_plan/runtime}")"
raw_root="$(realpath -m -- "$scratch_plan/raw")"
artifact_root="$(realpath -m -- "$scratch_plan/artifacts")"
provenance_file="$(realpath -m -- "$scratch_plan/provenance.json")"
out_root="$(realpath -m -- "${CODE_QUALITY_OUT_ROOT:-$root/evals/out/downstream-code-quality}")"
sanitized_output="$out_root/results.json"
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

assert_paths_do_not_overlap "$work_root" "$runtime_root"
assert_paths_do_not_overlap "$work_root" "$out_root"
assert_paths_do_not_overlap "$runtime_root" "$out_root"

validate_output_destination() {
  local node_command="$1"
  local mode="$2"
  "$node_command" - "$out_root" "$mode" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");

const outputRoot = path.resolve(process.argv[2]);
const mode = process.argv[3];
const marker = path.join(outputRoot, ".ai-plugins-code-quality-output");
const sanitizedOutput = path.join(outputRoot, "results.json");
const markerContents = "ai-plugins downstream code-quality output\n";

const entryExists = (entry) => {
  try {
    fs.lstatSync(entry);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
};

const validate = () => {
  if (mode !== "preflight" && mode !== "prepare") return 2;
  if (!entryExists(outputRoot)) {
    if (mode === "preflight") return 0;
    fs.mkdirSync(outputRoot, { recursive: true, mode: 0o700 });
  }

  const stat = fs.lstatSync(outputRoot);
  if (
    !stat.isDirectory() ||
    stat.isSymbolicLink() ||
    fs.realpathSync(outputRoot) !== outputRoot
  ) {
    return 2;
  }

  if (entryExists(marker)) {
    const markerStat = fs.lstatSync(marker);
    if (
      !markerStat.isFile() ||
      markerStat.isSymbolicLink() ||
      fs.realpathSync(marker) !== marker ||
      fs.readFileSync(marker, "utf8") !== markerContents
    ) {
      return 2;
    }
  } else {
    if (fs.readdirSync(outputRoot).length !== 0) return 2;
    if (mode === "prepare") {
      fs.writeFileSync(marker, markerContents, { flag: "wx", mode: 0o600 });
    }
  }

  if (entryExists(sanitizedOutput)) return 3;
  if (mode === "prepare") fs.chmodSync(outputRoot, 0o700);
  return 0;
};

try {
  process.exitCode = validate();
} catch (_error) {
  process.exitCode = 2;
}
NODE
}

report_output_destination_failure() {
  local status="$1"
  if [ "$status" -eq 3 ]; then
    echo 'code-quality benchmark output already exists' >&2
  else
    echo 'code-quality benchmark output root is not safely writable' >&2
  fi
}

resolve_nix_tool() {
  local name="$1"
  local candidate
  local canonical_directory
  local directory
  local old_ifs="$IFS"
  IFS=:
  for directory in $PATH; do
    [ -n "$directory" ] || continue
    candidate="$directory/$name"
    [ -x "$candidate" ] || continue
    canonical_directory="$(realpath "$directory")" || continue
    case "$canonical_directory" in
      /nix/store/*/bin)
        candidate="$canonical_directory/$name"
        [ -x "$candidate" ] || continue
        IFS="$old_ifs"
        printf '%s\n' "$candidate"
        return 0
        ;;
    esac
  done
  IFS="$old_ifs"
  printf 'required tool is not selected from Nix: %s\n' "$name" >&2
  return 2
}

safe_tool_names=(
  awk basename bash cat cargo cargo-clippy cargo-fmt cc chmod cp cut diff
  dirname env find git grep head ls mkdir mv pwd realpath rg rm rustc rustdoc
  rustfmt sed sha256sum sort tail tar touch tr wc xargs
)
runtime_tool_names=("${safe_tool_names[@]}" node prlimit sleep)
declare -A runtime_tool_paths=()

prepare_runtime_toolchain() {
  local closure_file="$1"
  local tool_directory
  local tool_name
  local tool_path
  local safe_tool_dirs=()

  runtime_tool_paths=()
  tool_records=()
  safe_store_roots=()
  for tool_name in "${runtime_tool_names[@]}"; do
    tool_path="$(resolve_nix_tool "$tool_name")"
    runtime_tool_paths["$tool_name"]="$tool_path"
    tool_directory="$(dirname "$tool_path")"
    safe_store_roots+=("${tool_directory%/bin}")
    tool_records+=("$tool_name:$(sha256sum "$tool_path" | cut -d' ' -f1)")
  done
  for tool_name in "${safe_tool_names[@]}"; do
    safe_tool_dirs+=("$(dirname "${runtime_tool_paths[$tool_name]}")")
  done
  safe_tool_path="$(printf '%s\n' "${safe_tool_dirs[@]}" | sort -u | paste -sd: -)"
  mapfile -t safe_store_roots < <(printf '%s\n' "${safe_store_roots[@]}" | LC_ALL=C sort -u)

  nix_store_bin="$(resolve_nix_tool nix-store)"
  "$nix_store_bin" --query --requisites "${safe_store_roots[@]}" |
    LC_ALL=C sort -u >"$closure_file"
  chmod 400 "$closure_file"
  nix_store_closure_sha256="$(sha256sum "$closure_file" | cut -d' ' -f1)"
  for safe_store_root in "${safe_store_roots[@]}"; do
    grep -Fxq -- "$safe_store_root" "$closure_file" || {
      echo 'Nix runtime closure is incomplete' >&2
      return 2
    }
  done
}

print_command() {
  printf '%q ' "$@"
  printf '\n'
}

print_plan() {
  for configured_case in "${case_ids[@]}"; do
    for sample in $(seq 1 "$samples"); do
      for mode in "${modes[@]}"; do
        workspace="$work_root/$configured_case/sample-$sample/$mode"
        printf 'workspace %s\n' "$workspace"
        printf 'provider openai-codex-sdk-%s workspace %s\n' \
          "$mode" "$workspace"
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
    printf 'gate provider-errors %s\n' \
      "$(jq -er '.diagnosticGates.providerErrors' "$contract")"
    printf 'gate operational-errors %s\n' \
      "$(jq -er '.diagnosticGates.operationalErrors' "$contract")"
    printf 'gate provenance-errors %s\n' \
      "$(jq -er '.diagnosticGates.provenanceErrors' "$contract")"
    printf 'gate safety-failures %s\n' \
      "$(jq -er '.diagnosticGates.safetyFailures' "$contract")"
  else
    echo 'diagnostic gates disabled: noncanonical run'
  fi

  print_command node "$workspace_preparer" "$work_root" \
    --case "$case_id" --samples "$samples"
  print_command node "$runtime_preparer" "$work_root/manifest.json" \
    "$runtime_root"
  printf 'execution EVAL_CASE_FILTER=%s EVAL_SAMPLES=%s\n' \
    "$case_id" "$samples"
  printf 'CODE_QUALITY_RUNTIME_MANIFEST=%s\n' \
    "$runtime_root/manifest.json"
  EVAL_OUT_DIR="$raw_root" \
    EVAL_TIMEOUT=20h \
    EVAL_CASE_FILTER="$case_id" \
    EVAL_SAMPLES="$samples" \
    "$root/scripts/evals/run.sh" --dry-run \
      "$benchmark_dir/promptfooconfig.yaml"
  print_command node "$secret_scanner" \
    "${secret_scan_options[@]}" "$raw_root" "$artifact_root"
  print_command node "$result_checker" \
    --results "$raw_root/results.json" \
    --artifacts "$artifact_root" \
    --runtime-manifest "$runtime_root/manifest.json" \
    --provenance "$provenance_file" \
    --output "$sanitized_output"
}

if [ "$runtime_preflight" -eq 1 ]; then
  preflight_root="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-code-quality-preflight.XXXXXX")"
  trap 'rm -rf -- "$preflight_root"' EXIT
  prepare_runtime_toolchain "$preflight_root/nix-store-closure"
  printf 'candidate-path %s\n' "$safe_tool_path"
  for tool_name in "${runtime_tool_names[@]}"; do
    printf 'runtime-tool %s %s\n' "$tool_name" "${runtime_tool_paths[$tool_name]}"
  done
  sed 's/^/closure /' "$preflight_root/nix-store-closure"
  exit 0
fi

if [ "$dry_run" -eq 1 ]; then
  print_plan
  exit 0
fi

if [ -z "${CODE_QUALITY_OPENAI_API_KEY:-}" ] ||
  [ "${#CODE_QUALITY_OPENAI_API_KEY}" -lt 20 ]; then
  echo 'CODE_QUALITY_OPENAI_API_KEY must contain a dedicated benchmark API key' >&2
  exit 2
fi
canonical_samples="$(jq -er '.sampleCount' "$contract")"
if [ "$samples" -ne "$canonical_samples" ]; then
  printf 'live execution requires the canonical sample count of %s\n' \
    "$canonical_samples" >&2
  exit 2
fi
if [ "${#case_ids[@]}" -ne "${#all_case_ids[@]}" ]; then
  echo 'live execution requires the complete configured case set' >&2
  exit 2
fi

for required in \
  "$workspace_preparer" \
  "$runtime_preparer" \
  "$boundary_launcher" \
  "$boundary_runtime" \
  "$codex_resolver" \
  "$secret_scanner" \
  "$result_checker"; do
  [ -f "$required" ] || {
    echo 'code-quality benchmark implementation is incomplete' >&2
    exit 2
  }
done

preflight_node_bin="$(resolve_nix_tool node)"
output_preflight_status=0
validate_output_destination "$preflight_node_bin" preflight ||
  output_preflight_status="$?"
if [ "$output_preflight_status" -ne 0 ]; then
  report_output_destination_failure "$output_preflight_status"
  exit 2
fi

"$root/scripts/evals/ensure-node-deps.sh" >/dev/null

scratch_root="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-code-quality-live.XXXXXX")"
chmod 700 "$scratch_root"
printf 'ai-plugins downstream code-quality run root\n' \
  >"$scratch_root/.ai-plugins-code-quality-run-root"
chmod 600 "$scratch_root/.ai-plugins-code-quality-run-root"
work_root="$scratch_root/workspaces"
runtime_root="$scratch_root/runtime"
raw_root="$scratch_root/raw"
artifact_root="$scratch_root/artifacts"
verifier_tmp_root="$scratch_root/verifier-tmp"
provenance_file="$scratch_root/provenance.json"
private_log="$scratch_root/promptfoo.log"
host_home="$scratch_root/host-home"
host_tmp="$scratch_root/host-tmp"
version_probe_home="$scratch_root/version-probe-home"
version_probe_tmp="$scratch_root/version-probe-tmp"
promptfoo_config_root="$scratch_root/promptfoo-config"
promptfoo_cache="$scratch_root/promptfoo-cache"
mkdir -m 700 \
  "$raw_root" \
  "$artifact_root" \
  "$verifier_tmp_root" \
  "$host_home" \
  "$host_tmp" \
  "$version_probe_home" \
  "$version_probe_tmp" \
  "$promptfoo_config_root" \
  "$promptfoo_cache"
scan_status=0
run_status=1
sanitized_created=0
cleanup_safe=1
node_bin=""
promptfoo_pid=""
promptfoo_launching=0
promptfoo_unit=""
systemctl_bin=""

run_trusted_systemctl() {
  "$timeout_bin" --signal=TERM --kill-after=2s 10s \
    "${runtime_tool_paths[env]}" -i \
      HOME="$host_home" \
      XDG_RUNTIME_DIR="/run/user/$UID" \
      PATH="$trusted_promptfoo_path" \
      LANG=C.UTF-8 \
      LC_ALL=C.UTF-8 \
      "$systemctl_bin" "$@"
}

promptfoo_process_is_running() {
  local active_pid="$1"
  local process_state=""

  kill -0 "$active_pid" 2>/dev/null || return 1
  if [ -r "/proc/$active_pid/stat" ]; then
    read -r _ _ process_state _ <"/proc/$active_pid/stat" || return 0
    [ "$process_state" != Z ] || return 1
  fi
  return 0
}

wait_for_promptfoo_exit() {
  local active_pid="$1"
  local attempts="$2"
  local attempt

  for ((attempt = 0; attempt < attempts; attempt += 1)); do
    promptfoo_process_is_running "$active_pid" || return 0
    "$sleep_bin" 0.1
  done
  ! promptfoo_process_is_running "$active_pid"
}

stop_promptfoo_scope() {
  local signal="$1"
  local active_pid="${promptfoo_pid:-}"
  local exited_after_signal=0
  local state=""
  local state_status=0
  local stop_status=0

  if [ -z "$active_pid" ] && [ "$promptfoo_launching" -eq 1 ]; then
    active_pid="${!:-}"
  fi
  if [ -n "$promptfoo_unit" ] && [ -x "$systemctl_bin" ]; then
    run_trusted_systemctl \
      --user kill --kill-whom=all "--signal=$signal" \
      "$promptfoo_unit" >/dev/null 2>&1 || true
  fi
  if [ -n "$active_pid" ] && wait_for_promptfoo_exit "$active_pid" 20; then
    exited_after_signal=1
  fi
  if [ "$exited_after_signal" -eq 0 ] &&
    [ -n "$promptfoo_unit" ] && [ -x "$systemctl_bin" ]; then
    run_trusted_systemctl --user stop "$promptfoo_unit" \
      >/dev/null 2>&1 || stop_status="$?"
  fi
  if [ -n "$active_pid" ] && promptfoo_process_is_running "$active_pid"; then
    kill -s "$signal" "$active_pid" 2>/dev/null || true
  fi
  if [ -n "$active_pid" ] && ! wait_for_promptfoo_exit "$active_pid" 100; then
    if [ -n "$promptfoo_unit" ] && [ -x "$systemctl_bin" ]; then
      run_trusted_systemctl \
        --user kill --kill-whom=all --signal=KILL \
        "$promptfoo_unit" >/dev/null 2>&1 || true
      run_trusted_systemctl --user stop "$promptfoo_unit" \
        >/dev/null 2>&1 || stop_status="$?"
    fi
    kill -KILL "$active_pid" 2>/dev/null || true
    wait_for_promptfoo_exit "$active_pid" 50 || return 1
  fi
  if [ -n "$active_pid" ]; then
    wait "$active_pid" 2>/dev/null || true
  fi
  if [ -n "$promptfoo_unit" ] && [ -x "$systemctl_bin" ]; then
    state="$(run_trusted_systemctl \
      --user show --property=ActiveState --value \
      "$promptfoo_unit" 2>/dev/null)" || state_status="$?"
    case "$state" in
      active | activating | deactivating | reloading) return 1 ;;
    esac
    if [ "$state_status" -ne 0 ] && [ "$stop_status" -ne 0 ]; then
      return 1
    fi
  fi
  promptfoo_pid=""
  promptfoo_launching=0
}

handle_promptfoo_interrupt() {
  local signal="$1"
  local status="$2"

  trap '' INT TERM
  run_status="$status"
  if ! stop_promptfoo_scope "$signal"; then
    cleanup_safe=0
    run_status=1
    echo 'code-quality benchmark cancellation could not confirm scope teardown' >&2
  fi
  exit "$run_status"
}

cleanup() {
  local status="$run_status"
  local candidate
  local exact_scan_paths=()
  local scan_paths=()
  trap - EXIT INT TERM
  if [ "$cleanup_safe" -ne 1 ]; then
    echo "preserving active benchmark scratch state: $scratch_root" >&2
    exit "$status"
  fi
  for candidate in \
    "$raw_root" \
    "$artifact_root" \
    "$host_home" \
    "$host_tmp" \
    "$promptfoo_config_root" \
    "$promptfoo_cache" \
    "$provenance_file" \
    "$private_log"; do
    [ ! -e "$candidate" ] || scan_paths+=("$candidate")
  done
  [ ! -e "$runtime_root" ] || exact_scan_paths+=("$runtime_root")
  if [ -n "$node_bin" ] &&
    { [ -x "$secret_scanner" ] || [ -f "$secret_scanner" ]; }; then
    if [ "${#scan_paths[@]}" -eq 0 ] ||
      ! "$node_bin" "$secret_scanner" \
        "${secret_scan_options[@]}" \
        "${scan_paths[@]}" >/dev/null 2>&1; then
      scan_status=1
      status=1
    fi
    if [ "${#exact_scan_paths[@]}" -gt 0 ] &&
      ! "$node_bin" "$secret_scanner" \
        --profile codex-runtime \
        --exact-only \
        "${secret_scan_options[@]}" \
        "${exact_scan_paths[@]}" >/dev/null 2>&1; then
      scan_status=1
      status=1
    fi
  else
    scan_status=1
    status=1
  fi
  rm -rf -- "$scratch_root"
  if [ "$scan_status" -ne 0 ] && [ "$sanitized_created" -eq 1 ]; then
    rm -f -- "$sanitized_output"
    sanitized_created=0
  fi
  if [ "$scan_status" -ne 0 ]; then
    echo 'code-quality benchmark secret scan failed' >&2
  fi
  exit "$status"
}
trap cleanup EXIT
trap 'handle_promptfoo_interrupt INT 130' INT
trap 'handle_promptfoo_interrupt TERM 143' TERM

nix_store_closure="$scratch_root/nix-store-closure"
prepare_runtime_toolchain "$nix_store_closure"
node_bin="${runtime_tool_paths[node]}"
prlimit_bin="${runtime_tool_paths[prlimit]}"
sleep_bin="${runtime_tool_paths[sleep]}"
trusted_promptfoo_path="$(dirname "$node_bin"):$PATH"
prlimit_sha256="$(sha256sum "$prlimit_bin" | cut -d' ' -f1)"
"$node_bin" "$workspace_preparer" "$work_root" \
  --case "$case_id" --samples "$samples" >/dev/null
workspace_manifest="$work_root/manifest.json"

bwrap_bin="$(resolve_nix_tool bwrap)"
timeout_bin="$(resolve_nix_tool timeout)"
systemd_run_bin="$(resolve_nix_tool systemd-run)"
systemctl_bin="$(resolve_nix_tool systemctl)"
scope_bash_bin="$(resolve_nix_tool bash)"
bwrap_sha256="$(sha256sum "$bwrap_bin" | cut -d' ' -f1)"
timeout_sha256="$(sha256sum "$timeout_bin" | cut -d' ' -f1)"
systemd_run_sha256="$(sha256sum "$systemd_run_bin" | cut -d' ' -f1)"
systemctl_sha256="$(sha256sum "$systemctl_bin" | cut -d' ' -f1)"

tool_records+=(
  "boundary-bwrap:$bwrap_sha256"
  "boundary-prlimit:$prlimit_sha256"
  "boundary-systemd-run:$systemd_run_sha256"
  "boundary-systemctl:$systemctl_sha256"
  "boundary-timeout:$timeout_sha256"
  "nix-store:$(sha256sum "$nix_store_bin" | cut -d' ' -f1)"
  "nix-store-closure:$nix_store_closure_sha256"
)

codex_resolution="$("$node_bin" "$codex_resolver")"
codex_bin="$(jq -er '.codexBin' <<<"$codex_resolution")"
codex_runtime_root="$(jq -er '.runtimeRoot' <<<"$codex_resolution")"
codex_runtime_manifest="$(jq -er '.runtimeManifest' <<<"$codex_resolution")"
codex_resource_bwrap="$(jq -er '.resourceBwrap' <<<"$codex_resolution")"
codex_resource_rg="$(jq -er '.resourceRg' <<<"$codex_resolution")"
codex_wrapper_version="$(jq -er '.wrapperVersion' <<<"$codex_resolution")"
codex_payload_version="$(jq -er '.payloadVersion' <<<"$codex_resolution")"
codex_resource_bwrap_sha256="$(sha256sum "$codex_resource_bwrap" | cut -d' ' -f1)"
codex_resource_rg_sha256="$(sha256sum "$codex_resource_rg" | cut -d' ' -f1)"
tool_records+=(
  "codex-wrapper:$codex_wrapper_version"
  "codex-payload:$codex_payload_version"
  "codex-resource-bwrap:$codex_resource_bwrap_sha256"
  "codex-resource-rg:$codex_resource_rg_sha256"
)
codex_expected_version="$(
  env -i \
    CODEX_HOME="$version_probe_home" \
    HOME="$version_probe_home" \
    TMPDIR="$version_probe_tmp" \
    PATH="$safe_tool_path" \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    "$codex_bin" --version
)"
codex_version="${codex_expected_version#codex-cli }"
[[ "$codex_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][A-Za-z0-9.-]+)?$ ]] || {
  echo 'package-native Codex version is invalid' >&2
  exit 2
}
codex_sha256="$(sha256sum "$codex_bin" | cut -d' ' -f1)"
node_expected_version="$("$node_bin" --version)"
node_version="${node_expected_version#v}"
[[ "$node_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][A-Za-z0-9.-]+)?$ ]] || {
  echo 'package-native Node version is invalid' >&2
  exit 2
}
node_sha256="$(sha256sum "$node_bin" | cut -d' ' -f1)"
package_versions="$("$node_bin" - "$root" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");
const root = process.argv[2];
const lock = JSON.parse(fs.readFileSync(path.join(root, "package-lock.json"), "utf8"));
const versions = {};
for (const name of ["@openai/codex-sdk", "promptfoo"]) {
  const packageFile = path.join(root, "node_modules", ...name.split("/"), "package.json");
  const installed = JSON.parse(fs.readFileSync(packageFile, "utf8"));
  const locked = lock.packages?.[`node_modules/${name}`];
  if (
    installed.name !== name ||
    typeof installed.version !== "string" ||
    !/^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$/.test(installed.version) ||
    locked?.version !== installed.version
  ) {
    throw new Error(`installed package does not match package-lock: ${name}`);
  }
  versions[name] = installed.version;
}
process.stdout.write(JSON.stringify(versions));
NODE
)"
sdk_version="$(jq -er '.["@openai/codex-sdk"]' <<<"$package_versions")"
promptfoo_version="$(jq -er '.promptfoo' <<<"$package_versions")"
package_lock_sha256="$(sha256sum "$root/package-lock.json" | cut -d' ' -f1)"
boundary_sha256="$("$node_bin" - "$boundary_launcher" "$boundary_runtime" <<'NODE'
const crypto = require("node:crypto");
const fs = require("node:fs");
const hash = crypto.createHash("sha256");
for (const file of process.argv.slice(2)) {
  const bytes = fs.readFileSync(file);
  const length = Buffer.alloc(8);
  length.writeBigUInt64BE(BigInt(bytes.length));
  hash.update(length).update(bytes);
}
process.stdout.write(hash.digest("hex"));
NODE
)"
tool_records+=(
  "codex-binary:$codex_sha256"
  "codex-version:$codex_version"
  "node-version:$node_version"
  "codex-sdk-version:$sdk_version"
  "promptfoo-version:$promptfoo_version"
  "package-lock:$package_lock_sha256"
)
toolchain_sha256="$(printf '%s\n' "${tool_records[@]}" | LC_ALL=C sort | sha256sum | cut -d' ' -f1)"
model="$(jq -er '.provider.model' "$contract")"
reasoning_effort="$(jq -er '.provider.reasoningEffort' "$contract")"

env -i \
  HOME="$host_home" \
  TMPDIR="$host_tmp" \
  PATH="$safe_tool_path" \
  LANG=C.UTF-8 \
  LC_ALL=C.UTF-8 \
  CODE_QUALITY_CODEX_REAL_BIN="$codex_bin" \
  CODE_QUALITY_CODEX_EXPECTED_SHA256="$codex_sha256" \
  CODE_QUALITY_CODEX_EXPECTED_VERSION="$codex_expected_version" \
  CODE_QUALITY_CODEX_MODEL="$model" \
  CODE_QUALITY_CODEX_REASONING_EFFORT="$reasoning_effort" \
  CODE_QUALITY_BOUNDARY_SHA256="$boundary_sha256" \
  CODE_QUALITY_TOOLCHAIN_COMPOSITION_SHA256="$toolchain_sha256" \
  "$node_bin" "$runtime_preparer" \
    "$workspace_manifest" "$runtime_root" >/dev/null
runtime_manifest="$runtime_root/manifest.json"

promptfoo_scope_entry="$scratch_root/promptfoo-scope-entry"
printf '%s\n' \
  "#!$scope_bash_bin" \
  'set -euo pipefail' \
  'unset XDG_RUNTIME_DIR' \
  'exec "$@"' \
  >"$promptfoo_scope_entry"
chmod 500 "$promptfoo_scope_entry"

"$node_bin" - \
  "$runtime_manifest" \
  "$provenance_file" \
  "$model" \
  "$reasoning_effort" \
  "$codex_version" \
  "$codex_sha256" \
  "$node_version" \
  "$node_sha256" \
  "$sdk_version" \
  "$promptfoo_version" \
  "$package_lock_sha256" \
  "$boundary_sha256" \
  "$toolchain_sha256" <<'NODE'
const fs = require("node:fs");
const runtime = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const provenance = {
  schemaVersion: 1,
  benchmarkId: runtime.benchmarkId,
  runId: runtime.runId,
  contractSha256: runtime.contractSha256,
  matrixHash: runtime.matrixHash,
  workspaceManifestSha256: runtime.workspaceManifestSha256,
  runtimeManifestSha256: require("node:crypto")
    .createHash("sha256")
    .update(fs.readFileSync(process.argv[2]))
    .digest("hex"),
  model: process.argv[4],
  reasoningEffort: process.argv[5],
  codexVersion: process.argv[6],
  codexBinarySha256: process.argv[7],
  nodeVersion: process.argv[8],
  nodeBinarySha256: process.argv[9],
  codexSdkVersion: process.argv[10],
  promptfooVersion: process.argv[11],
  packageLockSha256: process.argv[12],
  boundarySha256: process.argv[13],
  toolchainCompositionSha256: process.argv[14],
};
fs.writeFileSync(process.argv[3], `${JSON.stringify(provenance, null, 2)}\n`, {
  flag: "wx",
  mode: 0o600,
});
NODE

promptfoo_unit="ai-plugins-code-quality-promptfoo-$UID-$$.scope"
promptfoo_launching=1
set +e
env -i \
  HOME="$host_home" \
  XDG_CONFIG_HOME="$host_home/.config" \
  TMPDIR="$host_tmp" \
  PATH="$trusted_promptfoo_path" \
  LANG=C.UTF-8 \
  LC_ALL=C.UTF-8 \
  GIT_CONFIG_GLOBAL=/dev/null \
  GIT_CONFIG_NOSYSTEM=1 \
  OPENAI_API_KEY="$CODE_QUALITY_OPENAI_API_KEY" \
  CODEX_API_KEY="$CODE_QUALITY_OPENAI_API_KEY" \
  CODE_QUALITY_WORKSPACE_MANIFEST="$workspace_manifest" \
  CODE_QUALITY_RUNTIME_MANIFEST="$runtime_manifest" \
  CODE_QUALITY_VERIFIER_OUT_ROOT="$artifact_root" \
  CODE_QUALITY_VERIFIER_TMP_ROOT="$verifier_tmp_root" \
  CODE_QUALITY_CODEX_LAUNCHER="$boundary_launcher" \
  CODE_QUALITY_NODE_BIN="$node_bin" \
  CODE_QUALITY_BWRAP_BIN="$bwrap_bin" \
  CODE_QUALITY_BWRAP_EXPECTED_SHA256="$bwrap_sha256" \
  CODE_QUALITY_NIX_STORE_CLOSURE="$nix_store_closure" \
  CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256="$nix_store_closure_sha256" \
  CODE_QUALITY_TIMEOUT_BIN="$timeout_bin" \
  CODE_QUALITY_TIMEOUT_EXPECTED_SHA256="$timeout_sha256" \
  CODE_QUALITY_PRLIMIT_BIN="$prlimit_bin" \
  CODE_QUALITY_PRLIMIT_EXPECTED_SHA256="$prlimit_sha256" \
  CODE_QUALITY_SYSTEMD_RUN_BIN="$systemd_run_bin" \
  CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256="$systemd_run_sha256" \
  CODE_QUALITY_CODEX_REAL_BIN="$codex_bin" \
  CODE_QUALITY_CODEX_EXPECTED_SHA256="$codex_sha256" \
  CODE_QUALITY_CODEX_EXPECTED_VERSION="$codex_expected_version" \
  CODE_QUALITY_CODEX_RESOURCE_BWRAP_EXPECTED_SHA256="$codex_resource_bwrap_sha256" \
  CODE_QUALITY_CODEX_RG_EXPECTED_SHA256="$codex_resource_rg_sha256" \
  CODE_QUALITY_TOOL_PATH="$safe_tool_path" \
  CODE_QUALITY_WALL_TIMEOUT_SECONDS=7200 \
  CODE_QUALITY_OUTPUT_MAX_BYTES=16777216 \
  CODE_QUALITY_WORKSPACE_MAX_BYTES=536870912 \
  CODE_QUALITY_WORKSPACE_MAX_ENTRIES=50000 \
  CODE_QUALITY_CODEX_MODEL="$model" \
  CODE_QUALITY_CODEX_REASONING_EFFORT="$reasoning_effort" \
  AI_PLUGINS_BWRAP_BIN="$bwrap_bin" \
  AI_PLUGINS_PRLIMIT_BIN="$prlimit_bin" \
  EVAL_OUT_DIR="$raw_root" \
  EVAL_TIMEOUT=20h \
  EVAL_CASE_FILTER="$case_id" \
  EVAL_SAMPLES="$samples" \
  PROMPTFOO_MAX_CONCURRENCY=1 \
  PROMPTFOO_CONFIG_DIR="$promptfoo_config_root" \
  PROMPTFOO_CACHE_PATH="$promptfoo_cache" \
  PROMPTFOO_DISABLE_TELEMETRY=1 \
  XDG_RUNTIME_DIR="/run/user/$UID" \
  "$systemd_run_bin" \
    --user \
    --scope \
    --quiet \
    --collect \
    --expand-environment=false \
    "--unit=$promptfoo_unit" \
    --property=MemoryMax=12884901888 \
    --property=MemorySwapMax=0 \
    --property=TasksMax=768 \
    --property=CPUQuota=600% \
    --property=OOMPolicy=kill \
    --property=KillMode=control-group \
    -- \
    "$promptfoo_scope_entry" \
    "$root/scripts/evals/run.sh" "$benchmark_dir/promptfooconfig.yaml" \
  >"$private_log" 2>&1 &
promptfoo_pid="$!"
promptfoo_launching=0
wait "$promptfoo_pid"
promptfoo_status="$?"
promptfoo_pid=""
set -e

"$node_bin" "$secret_scanner" \
  "${secret_scan_options[@]}" \
  "$raw_root" "$artifact_root" "$private_log"

prepare_output_status=0
validate_output_destination "$node_bin" prepare || prepare_output_status="$?"
if [ "$prepare_output_status" -ne 0 ]; then
  report_output_destination_failure "$prepare_output_status"
  exit 2
fi

checker_status=0
"$node_bin" "$result_checker" \
  --results "$raw_root/results.json" \
  --artifacts "$artifact_root" \
  --runtime-manifest "$runtime_manifest" \
  --provenance "$provenance_file" \
  --output "$sanitized_output" || checker_status="$?"
[ ! -f "$sanitized_output" ] || sanitized_created=1

if [ "$sanitized_created" -eq 1 ]; then
  if ! "$node_bin" "$secret_scanner" \
    "${secret_scan_options[@]}" \
    "$sanitized_output"; then
    rm -f -- "$sanitized_output"
    sanitized_created=0
    echo 'sanitized benchmark output failed secret scanning' >&2
    exit 1
  fi
fi

if [ "$promptfoo_status" -ge 128 ]; then
  echo 'code-quality benchmark provider execution was interrupted' >&2
  run_status="$promptfoo_status"
elif [ "$checker_status" -ne 0 ]; then
  echo 'code-quality benchmark result validation failed' >&2
  run_status="$checker_status"
else
  printf 'code-quality benchmark complete: %s\n' "$sanitized_output"
  run_status=0
fi
exit "$run_status"
