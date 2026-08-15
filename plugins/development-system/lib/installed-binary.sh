#!/usr/bin/env bash

# Shared resolution for Development System's host-local Rust executables.

development_system_data_home() {
  if [[ "${XDG_DATA_HOME:-}" == /* ]]; then
    printf '%s\n' "$XDG_DATA_HOME"
    return 0
  fi
  if [[ "${HOME:-}" == /* ]]; then
    printf '%s/.local/share\n' "$HOME"
    return 0
  fi
  return 1
}

development_system_plugin_version() {
  local plugin_root=$1
  local version
  version="$(sed -nE 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"([^"]+)"[[:space:]]*,?[[:space:]]*$/\1/p' \
    "$plugin_root/.codex-plugin/plugin.json" | head -n 1)"
  [[ -n "$version" ]] || return 1
  printf '%s\n' "$version"
}

development_system_host() {
  local os architecture
  os="$(uname -s)" || return 1
  architecture="$(uname -m)" || return 1
  printf '%s-%s\n' \
    "$(printf '%s' "$os" | tr '[:upper:]' '[:lower:]')" \
    "$(printf '%s' "$architecture" | tr '[:upper:]' '[:lower:]')"
}

development_system_installed_binary_path() {
  local plugin_root=$1
  local binary_name=$2
  local data_home version host
  data_home="$(development_system_data_home)" || return 1
  version="$(development_system_plugin_version "$plugin_root")" || return 1
  host="$(development_system_host)" || return 1
  printf '%s/ai-plugins/development-system/%s/%s/%s\n' \
    "$data_home" "$version" "$host" "$binary_name"
}

development_system_exec_installed_binary() {
  local plugin_root=$1
  local binary_name=$2
  shift 2
  local binary_path data_home version host
  data_home="$(development_system_data_home)" || {
    printf '%s\n' "development_system.binary_install_location_unavailable remediation=set_XDG_DATA_HOME_or_HOME" >&2
    exit 1
  }
  version="$(development_system_plugin_version "$plugin_root")" || {
    printf '%s\n' "development_system.plugin_version_unavailable plugin_root=$plugin_root" >&2
    exit 1
  }
  host="$(development_system_host)" || {
    printf '%s\n' "development_system.host_unavailable" >&2
    exit 1
  }
  binary_path="$data_home/ai-plugins/development-system/$version/$host/$binary_name"
  if [[ ! -x "$binary_path" ]]; then
    printf '%s\n' \
      "development_system.binary_missing binary=$binary_path remediation='run the Development System setup skill, or from the marketplace checkout run: just install-development-system-binaries; requires Cargo'" >&2
    exit 1
  fi
  exec "$binary_path" "$@"
}
