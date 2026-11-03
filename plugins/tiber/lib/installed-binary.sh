#!/usr/bin/env bash

# Shared resolution for Tiber's host-local Rust executables.

tiber_data_home() {
  if [[ "${PLUGIN_DATA:-}" == /* ]]; then
    printf '%s\n' "$PLUGIN_DATA"
    return 0
  fi
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

tiber_plugin_version() {
  local plugin_root=$1
  local version
  version="$(sed -nE 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"([^"]+)"[[:space:]]*,?[[:space:]]*$/\1/p' \
    "$plugin_root/plugin.json" | head -n 1)"
  [[ -n "$version" ]] || return 1
  printf '%s\n' "$version"
}

tiber_host() {
  local os architecture
  os="$(uname -s)" || return 1
  architecture="$(uname -m)" || return 1
  printf '%s-%s\n' \
    "$(printf '%s' "$os" | tr '[:upper:]' '[:lower:]')" \
    "$(printf '%s' "$architecture" | tr '[:upper:]' '[:lower:]')"
}

tiber_effective_host() {
  if [[ -n "${TIBER_HOST_OVERRIDE:-}" ]]; then
    printf '%s\n' "$TIBER_HOST_OVERRIDE"
  else
    tiber_host
  fi
}

tiber_installation_matches_plugin() {
  local plugin_root=$1
  local data_home version host installation
  data_home="$(tiber_data_home)" || return 1
  version="$(tiber_plugin_version "$plugin_root")" || return 1
  host="$(tiber_effective_host)" || return 1
  installation="$data_home/ai-plugins/tiber/$version/$host"
  [[ -x "$installation/tiber" ]] || return 1
  [[ -f "$installation/.plugin-version" ]] || return 1
  [[ "$(<"$installation/.plugin-version")" == "$version" ]]
}

tiber_installed_binary_path() {
  local plugin_root=$1
  local binary_name=$2
  local data_home version host
  data_home="$(tiber_data_home)" || return 1
  version="$(tiber_plugin_version "$plugin_root")" || return 1
  host="$(tiber_effective_host)" || return 1
  printf '%s/ai-plugins/tiber/%s/%s/%s\n' \
    "$data_home" "$version" "$host" "$binary_name"
}

tiber_exec_installed_binary() {
  local plugin_root=$1
  local binary_name=$2
  shift 2
  local binary_path data_home version host
  data_home="$(tiber_data_home)" || {
    printf '%s\n' "tiber.binary_install_location_unavailable remediation=set_XDG_DATA_HOME_or_HOME" >&2
    exit 1
  }
  version="$(tiber_plugin_version "$plugin_root")" || {
    printf '%s\n' "tiber.plugin_version_unavailable plugin_root=$plugin_root" >&2
    exit 1
  }
  host="$(tiber_effective_host)" || {
    printf '%s\n' "tiber.host_unavailable" >&2
    exit 1
  }
  binary_path="$data_home/ai-plugins/tiber/$version/$host/$binary_name"
  if ! tiber_installation_matches_plugin "$plugin_root"; then
    "$plugin_root/scripts/install-tiber-binaries.sh" --auto || {
      printf '%s\n' "tiber.binary_repair_failed binary=$binary_name version=$version" >&2
      exit 1
    }
  fi
  if [[ ! -x "$binary_path" ]]; then
    printf '%s\n' \
      "tiber.binary_missing binary=$binary_path remediation='run scripts/install-tiber-binaries.sh --auto'" >&2
    exit 1
  fi
  if [[ "$binary_name" == tiber ]] && [[ "${TIBER_AGENT_FORWARDING:-}" != current-session ]] && [[ -z "${SSH_AUTH_SOCK:-}" ]]; then
    local config_home socket_file socket
    config_home="${XDG_CONFIG_HOME:-${HOME:-}/.config}"
    if [[ "${PLUGIN_DATA:-}" == /* ]]; then
      socket_file="$PLUGIN_DATA/signing-agent-socket"
    else
      socket_file="$config_home/ai-plugins/tiber/signing-agent-socket"
    fi
    if [[ -f "$socket_file" ]]; then
      IFS= read -r socket <"$socket_file" || true
      if [[ "$socket" == /* ]] && [[ -S "$socket" ]]; then
        export SSH_AUTH_SOCK="$socket"
      fi
    fi
  fi
  exec "$binary_path" "$@"
}
