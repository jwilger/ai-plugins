#!/usr/bin/env bash
set -euo pipefail

# Invoke this exact one-operation helper through the host's supported approval
# mechanism. A rejected approval is final; this script never elevates itself.
plugin_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$plugin_root/lib/installed-binary.sh"
if [[ $# -ne 2 ]]; then
  printf '%s\n' "usage: replay-review-operation.sh REPOSITORY_ROOT FINAL_REVIEW_TOOL < arguments.json" >&2
  exit 2
fi
if ! development_system_installation_matches_plugin "$plugin_root"; then
  printf '%s\n' "development_system.replay_binary_version_mismatch retryable=install_matching_plugin_binaries_and_restart_harness" >&2
  exit 1
fi
binary="$(development_system_installed_binary_path "$plugin_root" development-discipline-mcp)"
# Preserve the configured SSH signing agent just as the task-board launcher does.
if [[ -z "${SSH_AUTH_SOCK:-}" ]]; then
  if [[ "${PLUGIN_DATA:-}" == /* ]]; then
    socket_file="$PLUGIN_DATA/signing-agent-socket"
  else
    socket_file="${XDG_CONFIG_HOME:-${HOME:-}/.config}/ai-plugins/development-system/signing-agent-socket"
  fi
  if [[ -f "$socket_file" ]]; then
    IFS= read -r socket <"$socket_file" || true
    if [[ "${socket:-}" == /* ]] && [[ -S "$socket" ]]; then
      export SSH_AUTH_SOCK="$socket"
    fi
  fi
fi
exec "$binary" --replay-review-operation "$1" "$2"
