#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'EVAL_PROVIDER_BOUNDARY_ERROR:%s\n' "$1" >&2
  exit 64
}

require_absolute_directory() {
  local value="$1"
  local label="$2"
  [ -n "$value" ] || fail "missing-$label"
  case "$value" in /*) ;; *) fail "$label-not-absolute" ;; esac
  [ -d "$value" ] || fail "$label-not-directory"
  [ "$(realpath -- "$value")" = "$value" ] || fail "$label-not-canonical"
}

require_absolute_executable() {
  local value="$1"
  local label="$2"
  [ -n "$value" ] || fail "missing-$label"
  case "$value" in /*) ;; *) fail "$label-not-absolute" ;; esac
  [ -f "$value" ] && [ -x "$value" ] || fail "$label-not-executable"
  [ "$(realpath -- "$value")" = "$value" ] || fail "$label-not-canonical"
}

require_absolute_file() {
  local value="$1"
  local label="$2"
  [ -n "$value" ] || fail "missing-$label"
  case "$value" in /*) ;; *) fail "$label-not-absolute" ;; esac
  [ -f "$value" ] && [ -r "$value" ] || fail "$label-not-readable"
  [ "$(realpath -- "$value")" = "$value" ] || fail "$label-not-canonical"
}

is_same_or_ancestor() {
  local ancestor="$1"
  local descendant="$2"
  [ "$ancestor" = "$descendant" ] || [[ "$descendant" == "$ancestor"/* ]]
}

paths_overlap() {
  is_same_or_ancestor "$1" "$2" || is_same_or_ancestor "$2" "$1"
}

store_root_for_path() {
  local value="$1"
  local relative
  case "$value" in
    /nix/store/*)
      relative="${value#/nix/store/}"
      printf '/nix/store/%s\n' "${relative%%/*}"
      ;;
    *) return 1 ;;
  esac
}

append_store_root() {
  local value="$1"
  local store_root
  local existing
  store_root="$(store_root_for_path "$value")" || return 0
  [ -d "$store_root" ] || fail runtime-store-root-unavailable
  for existing in "${runtime_roots[@]:-}"; do
    [ "$existing" != "$store_root" ] || return 0
  done
  runtime_roots+=("$store_root")
}

harness="${EVAL_PROVIDER_HARNESS:-}"
mode="${EVAL_PLUGIN_MODE:-}"
home="${EVAL_PROVIDER_HOME:-}"
workspace="${EVAL_PROVIDER_WORKSPACE:-}"
bwrap_bin="${EVAL_PROVIDER_BWRAP_BIN:-}"
plugin_snapshot="${EVAL_PROVIDER_PLUGIN_SNAPSHOT:-}"
repo_root="$(realpath -- "$(dirname "${BASH_SOURCE[0]}")/../..")"
plugin_source_root="$repo_root/plugins/development-system"

case "$harness" in codex | claude) ;; *) fail unsupported-harness ;; esac
case "$mode" in no-plugins | development-system) ;; *) fail unsupported-plugin-mode ;; esac
require_absolute_directory "$home" provider-home
require_absolute_directory "$workspace" provider-workspace
require_absolute_executable "$bwrap_bin" bwrap

paths_overlap "$workspace" "$home" && fail workspace-not-isolated
paths_overlap "$workspace" "$repo_root" && fail workspace-overlaps-repository-source
paths_overlap "$home" "$plugin_source_root" && fail provider-home-overlaps-plugin-source
[ ! -e "$workspace/.git" ] || fail workspace-contains-repository-metadata
[ ! -e "$home/.git" ] || fail provider-home-contains-repository-metadata

if [ "$harness" = claude ]; then
  home_marker="$home/.ai-plugins-claude-eval-home"
  expected_home_marker="ai-plugins Claude eval home"
else
  home_marker="$home/.ai-plugins-eval-home"
  expected_home_marker="ai-plugins Codex eval home"
fi
[ -f "$home_marker" ] || fail provider-home-not-sanitized
home_marker_lines=()
mapfile -t home_marker_lines <"$home_marker"
[ "${#home_marker_lines[@]}" -eq 1 ] || fail provider-home-marker-invalid
[ "${home_marker_lines[0]}" = "$expected_home_marker" ] || fail provider-home-marker-invalid

if [ "$mode" = no-plugins ]; then
  [ -z "$plugin_snapshot" ] || fail no-plugin-condition-has-plugin-snapshot
else
  require_absolute_directory "$plugin_snapshot" plugin-snapshot
  case "$plugin_snapshot" in
    "$home"/*) ;;
    *) fail plugin-snapshot-outside-condition-home ;;
  esac
  paths_overlap "$plugin_snapshot" "$plugin_source_root" && fail plugin-snapshot-overlaps-plugin-source
  [ ! -e "$plugin_snapshot/.git" ] || fail plugin-snapshot-contains-repository-metadata
fi

# The hook writes its marker inside the disposable clone. Keep the host-side
# evidence outside that clone, so completed invocations prove hook execution
# without sharing mutable state between concurrent conditions.
session_start_evidence=""
if [ "$mode" = development-system ]; then
  session_start_evidence="${EVAL_PROVIDER_SESSION_START_EVIDENCE:-}"
  [ -n "$session_start_evidence" ] || fail missing-session-start-evidence
  case "$session_start_evidence" in /*) ;; *) fail session-start-evidence-not-absolute ;; esac
  session_start_evidence="$(realpath -m -- "$session_start_evidence")"
  case "$session_start_evidence" in "$home"/session-start-marker) ;; *) fail unsafe-session-start-evidence ;; esac
  [ -d "$(dirname -- "$session_start_evidence")" ] || fail session-start-evidence-parent-unavailable
fi

invocation_root="$(mktemp -d /tmp/ai-plugins-provider-boundary.XXXXXXXX)" || fail invocation-root-unavailable
case "$invocation_root" in /tmp/ai-plugins-provider-boundary.*) ;; *) fail unsafe-invocation-root ;; esac
chmod 700 "$invocation_root"
cleanup() {
  rm -rf -- "$invocation_root"
}
trap cleanup EXIT

invocation_home="$invocation_root/home"
invocation_workspace="$invocation_root/workspace"
mkdir -m 700 "$invocation_home" "$invocation_workspace"
cp -a -- "$home/." "$invocation_home/" || fail provider-home-clone-failed
cp -a -- "$workspace/." "$invocation_workspace/" || fail provider-workspace-clone-failed

runtime_plugin_snapshot=""
if [ "$mode" = development-system ]; then
  plugin_relative="${plugin_snapshot#"$home"/}"
  runtime_plugin_snapshot="$invocation_home/$plugin_relative"
  require_absolute_directory "$runtime_plugin_snapshot" cloned-plugin-snapshot
fi

env_bin="$(realpath -- "$(command -v env)")"
bash_bin="$(realpath -- "$(command -v bash)")"
dirname_bin="$(realpath -- "$(command -v dirname)")"
sleep_bin="$(realpath -- "$(command -v sleep)")"
require_absolute_executable "$env_bin" env-runtime
require_absolute_executable "$bash_bin" bash-runtime
require_absolute_executable "$dirname_bin" dirname-runtime
require_absolute_executable "$sleep_bin" sleep-runtime
ca_bundle="$(realpath -- /etc/ssl/certs/ca-certificates.crt)"
require_absolute_file "$ca_bundle" ca-bundle

nix_ld=""
nix_ld_library_path=""
if [ -n "${NIX_LD:-}" ]; then
  nix_ld="$(realpath -- "$NIX_LD")"
  case "$nix_ld" in /nix/store/*) ;; *) fail unsafe-nix-ld ;; esac
  [ -f "$nix_ld" ] && [ -x "$nix_ld" ] || fail nix-ld-unavailable
fi
if [ -n "${NIX_LD_LIBRARY_PATH:-}" ]; then
  old_ifs="$IFS"
  IFS=:
  for entry in $NIX_LD_LIBRARY_PATH; do
    resolved_entry="$(realpath -- "$entry")"
    case "$resolved_entry" in /nix/store/*) ;; *) fail unsafe-nix-ld-library-path ;; esac
    if [ -z "$nix_ld_library_path" ]; then
      nix_ld_library_path="$resolved_entry"
    else
      nix_ld_library_path="$nix_ld_library_path:$resolved_entry"
    fi
  done
  IFS="$old_ifs"
fi

real_provider=""
node_bin=""
codex_runtime=""
if [ "$harness" = claude ]; then
  real_provider="${EVAL_PROVIDER_REAL_BIN:-}"
  require_absolute_executable "$real_provider" claude
else
  node_bin="${EVAL_PROVIDER_NODE_BIN:-}"
  codex_runtime="${EVAL_PROVIDER_CODEX_RUNTIME:-}"
  require_absolute_executable "$node_bin" node
  require_absolute_directory "$codex_runtime" codex-runtime
  [ -f "$codex_runtime/codex/bin/codex.js" ] || fail codex-entrypoint-unavailable
fi

runtime_roots=()
append_store_root "$env_bin"
append_store_root "$bash_bin"
append_store_root "$dirname_bin"
append_store_root "$sleep_bin"
append_store_root "$ca_bundle"
[ -z "$nix_ld" ] || append_store_root "$nix_ld"
if [ -n "$nix_ld_library_path" ]; then
  old_ifs="$IFS"
  IFS=:
  for entry in $nix_ld_library_path; do append_store_root "$entry"; done
  IFS="$old_ifs"
fi
[ -z "$real_provider" ] || append_store_root "$real_provider"
[ -z "$node_bin" ] || append_store_root "$node_bin"

nix_store_bin="$(command -v nix-store)"
case "$nix_store_bin" in /nix/store/*/bin/nix-store) ;; *) fail unsafe-nix-store ;; esac
[ -x "$nix_store_bin" ] || fail nix-store-not-executable
closure_manifest="$invocation_root/runtime-closure"
"$nix_store_bin" -qR "${runtime_roots[@]}" >"$closure_manifest" || fail runtime-closure-unavailable
[ -s "$closure_manifest" ] || fail runtime-closure-empty

boundary=(
  "$bwrap_bin"
  --die-with-parent
  --new-session
  --unshare-all
  --share-net
  --clearenv
  --tmpfs /
  --proc /proc
  --dev /dev
  --tmpfs /tmp
  --dir /etc
  --dir /bin
  --dir /usr
  --dir /usr/bin
  --dir /nix
  --dir /nix/store
  --dir /lib64
  --dir /runtime
  --dir /etc/ssl
  --dir /etc/ssl/certs
  --ro-bind "$(realpath -- /etc/resolv.conf)" /etc/resolv.conf
  --ro-bind "$(realpath -- /etc/hosts)" /etc/hosts
  --ro-bind "$ca_bundle" /etc/ssl/certs/ca-certificates.crt
  --symlink "$env_bin" /usr/bin/env
  --symlink "$bash_bin" /bin/bash
  --symlink "$bash_bin" /bin/sh
  --symlink "$bash_bin" /usr/bin/bash
  --symlink "$dirname_bin" /usr/bin/dirname
  --symlink "$sleep_bin" /usr/bin/sleep
  --bind "$invocation_home" /runtime/home
  --bind "$invocation_workspace" /workspace
  --chdir /workspace
  --setenv HOME /runtime/home
  --setenv PATH /bin:/usr/bin
  --setenv LANG "${LANG:-C.UTF-8}"
  --setenv LC_ALL "${LC_ALL:-C.UTF-8}"
  --setenv TERM "${TERM:-dumb}"
  --setenv EVAL_PLUGIN_MODE "$mode"
  --setenv SSL_CERT_FILE /etc/ssl/certs/ca-certificates.crt
)

while IFS= read -r closure_path; do
  case "$closure_path" in /nix/store/*) ;; *) fail unsafe-runtime-closure-entry ;; esac
  [ -d "$closure_path" ] || fail runtime-closure-entry-unavailable
  [ "$(store_root_for_path "$closure_path")" = "$closure_path" ] || fail runtime-closure-entry-not-root
  boundary+=(--ro-bind "$closure_path" "$closure_path")
done <"$closure_manifest"

if [ -n "$nix_ld" ]; then
  boundary+=(--symlink "$nix_ld" /lib64/ld-linux-x86-64.so.2)
fi

[ -z "$nix_ld" ] || boundary+=(--setenv NIX_LD "$nix_ld")
[ -z "$nix_ld_library_path" ] || boundary+=(--setenv NIX_LD_LIBRARY_PATH "$nix_ld_library_path")

forwarded_environment=(HTTPS_PROXY HTTP_PROXY ALL_PROXY NO_PROXY)
if [ "$harness" = claude ]; then
  forwarded_environment+=(ANTHROPIC_API_KEY CLAUDE_CODE_OAUTH_TOKEN)
else
  forwarded_environment+=(OPENAI_API_KEY CODEX_API_KEY)
fi
for variable in "${forwarded_environment[@]}"; do
  if [ -n "${!variable:-}" ]; then
    boundary+=(--setenv "$variable" "${!variable}")
  fi
done

if [ "$mode" = development-system ]; then
  if [ "$harness" = claude ]; then
    boundary+=(--ro-bind "$runtime_plugin_snapshot" /runtime/plugin)
  else
    boundary+=(--ro-bind "$runtime_plugin_snapshot" /runtime/marketplace)
  fi
fi

if [ "$harness" = claude ]; then
  boundary+=(
    --ro-bind "$real_provider" /runtime/claude
    --setenv CLAUDE_CONFIG_DIR /runtime/home/config
    --setenv CLAUDE_CODE_PLUGIN_CACHE_DIR /runtime/home/plugin-cache
    --setenv CLAUDE_CODE_DISABLE_AUTO_MEMORY 1
  )
  if [ "$mode" = development-system ]; then
    boundary+=(--setenv DEVELOPMENT_SYSTEM_EVAL_SESSION_START_MARKER /runtime/home/session-start-marker)
  fi
  if "${boundary[@]}" /runtime/claude "$@"; then
    provider_status=0
  else
    provider_status=$?
  fi
  if [ -n "$session_start_evidence" ] && [ -f "$invocation_home/session-start-marker" ]; then
    : >"$session_start_evidence"
  fi
  exit "$provider_status"
fi

boundary+=(
  --dir /runtime/node_modules
  --ro-bind "$node_bin" /runtime/node
  --ro-bind "$codex_runtime" /runtime/node_modules/@openai
  --setenv CODEX_HOME /runtime/home
)
if [ "$mode" = development-system ]; then
  boundary+=(--setenv DEVELOPMENT_SYSTEM_EVAL_SESSION_START_MARKER /runtime/home/session-start-marker)
fi
if [ "$#" -eq 0 ] || [ "$1" != exec ]; then
  fail codex-provider-supports-only-exec
fi
shift
codex_args=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --cd)
      [ "$#" -ge 2 ] || fail codex-cd-missing-value
      [ "$2" = "$workspace" ] || fail codex-cd-outside-condition-workspace
      codex_args+=(--cd /workspace)
      shift 2
      ;;
    --add-dir | --image | --output-schema)
      fail codex-unsupported-host-path-argument
      ;;
    *)
      codex_args+=("$1")
      shift
      ;;
  esac
done
if "${boundary[@]}" /runtime/node /runtime/node_modules/@openai/codex/bin/codex.js exec --dangerously-bypass-hook-trust "${codex_args[@]}"; then
  provider_status=0
else
  provider_status=$?
fi
if [ -n "$session_start_evidence" ] && [ -f "$invocation_home/session-start-marker" ]; then
  : >"$session_start_evidence"
fi
exit "$provider_status"
