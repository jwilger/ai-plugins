#!/usr/bin/env bash
set -euo pipefail

plugin_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
marketplace_root="$(cd -- "$plugin_root/../.." && pwd -P)"
source "$plugin_root/lib/installed-binary.sh"

if ! command -v cargo >/dev/null 2>&1 || ! cargo --version >/dev/null 2>&1; then
  printf '%s\n' "development_system.cargo_unavailable remediation=install_or_activate_a_working_Cargo_environment" >&2
  exit 1
fi

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

version_dir="$data_home/ai-plugins/development-system/$version"
mkdir -p "$version_dir"
if ! command -v flock >/dev/null 2>&1; then
  printf '%s\n' "development_system.flock_unavailable remediation=install_or_activate_a_working_flock" >&2
  exit 1
fi
exec 9>"$version_dir/.install.lock"
flock -x 9
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  build_dir="$CARGO_TARGET_DIR"
else
  build_dir="$data_home/ai-plugins/development-system/build-cache/$version/$host"
fi
if [[ "$build_dir" != /* ]]; then
  build_dir="$marketplace_root/$build_dir"
fi
mkdir -p "$build_dir"
staged_dir=""
staged_link=""
previous_target=""
host_link="$version_dir/$host"
cleanup() {
  if [[ -n "$staged_link" ]]; then
    rm -f -- "$staged_link"
  fi
  if [[ -n "$staged_dir" ]]; then
    if [[ -L "$host_link" ]] && [[ "$(readlink "$host_link")" == "$(basename "$staged_dir")" ]]; then
      : # The atomic rename published this directory before an interruption.
    else
      rm -rf -- "$staged_dir"
    fi
  fi
}
trap cleanup EXIT

# Cargo discovers each component's pinned rust-toolchain.toml. Both lockfiles
# are enforced; the repository Nix devshell is optional.
(
  cd "$plugin_root/components/tiber/rust"
  CARGO_TARGET_DIR="$build_dir/tiber" cargo build --locked --release --bin tiber
)
(
  cd "$plugin_root/components/development-discipline/rust"
  CARGO_TARGET_DIR="$build_dir/development-discipline" \
    cargo build --locked --release --bin development-discipline-mcp
)

staged_dir="$(mktemp -d "$version_dir/.${host}.staging.XXXXXX")"
install -m 0755 "$build_dir/tiber/release/tiber" "$staged_dir/tiber"
install -m 0755 \
  "$build_dir/development-discipline/release/development-discipline-mcp" \
  "$staged_dir/development-discipline-mcp"

staged_link="$(mktemp "$version_dir/.${host}.link.XXXXXX")"
rm -f -- "$staged_link"
ln -s "$(basename "$staged_dir")" "$staged_link"

if [[ -L "$host_link" ]]; then
  previous_target="$(readlink "$host_link")"
fi

if mv -fT "$staged_link" "$host_link" 2>/dev/null; then
  :
elif mv -fh "$staged_link" "$host_link" 2>/dev/null; then
  :
else
  printf '%s\n' "development_system.atomic_install_unavailable host=$host" >&2
  exit 1
fi
staged_link=""
staged_dir=""

if [[ "$previous_target" == ".${host}.staging."* ]]; then
  rm -rf -- "$version_dir/$previous_target"
fi

printf '%s\n' \
  "development_system.binaries_installed version=$version host=$host directory=$version_dir/$host"
