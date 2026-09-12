#!/usr/bin/env bash
set -euo pipefail

plugin_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
marketplace_root="$(cd -- "$plugin_root/../.." && pwd -P)"
source "$plugin_root/lib/installed-binary.sh"

from_source=false
case "${1:-}" in
  "") ;;
  --from-source)
    from_source=true
    shift
    ;;
  *)
    printf '%s\n' "development_system.install_usage usage='$0 [--from-source]'" >&2
    exit 2
    ;;
esac
if [[ $# -ne 0 ]]; then
  printf '%s\n' "development_system.install_usage usage='$0 [--from-source]'" >&2
  exit 2
fi

data_home="$(development_system_data_home)" || {
  printf '%s\n' "development_system.binary_install_location_unavailable remediation=set_XDG_DATA_HOME_or_HOME" >&2
  exit 1
}
version="$(development_system_plugin_version "$plugin_root")" || {
  printf '%s\n' "development_system.plugin_version_unavailable plugin_root=$plugin_root" >&2
  exit 1
}
host="${DEVELOPMENT_SYSTEM_HOST_OVERRIDE:-$(development_system_host)}" || {
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
staged_dir=""
staged_link=""
previous_target=""
download_dir=""
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
  if [[ -n "$download_dir" ]]; then
    rm -rf -- "$download_dir"
  fi
}
trap cleanup EXIT

staged_dir="$(mktemp -d "$version_dir/.${host}.staging.XXXXXX")"
if [[ "$from_source" == true ]]; then
  if ! command -v cargo >/dev/null 2>&1 || ! cargo --version >/dev/null 2>&1; then
    printf '%s\n' "development_system.cargo_unavailable remediation=install_or_activate_a_working_Cargo_environment" >&2
    exit 1
  fi
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    build_dir="$CARGO_TARGET_DIR"
  else
    build_dir="$data_home/ai-plugins/development-system/build-cache/$version/$host"
  fi
  if [[ "$build_dir" != /* ]]; then
    build_dir="$marketplace_root/$build_dir"
  fi
  mkdir -p "$build_dir"

  # Cargo discovers each component's pinned rust-toolchain.toml. Both
  # lockfiles are enforced; the repository Nix devshell is optional.
  (
    cd "$plugin_root/components/tiber/rust"
    CARGO_TARGET_DIR="$build_dir/tiber" cargo build --locked --release --bin tiber
  )
  (
    cd "$plugin_root/components/development-discipline/rust"
    CARGO_TARGET_DIR="$build_dir/development-discipline" \
      cargo build --locked --release --bin development-discipline-mcp
  )
  install -m 0755 "$build_dir/tiber/release/tiber" "$staged_dir/tiber"
  install -m 0755 \
    "$build_dir/development-discipline/release/development-discipline-mcp" \
    "$staged_dir/development-discipline-mcp"
else
  if [[ "$host" != linux-x86_64 ]]; then
    printf '%s\n' \
      "development_system.release_host_unsupported host=$host remediation='rerun with --from-source'" >&2
    exit 1
  fi
  for dependency in curl tar sha256sum; do
    if ! command -v "$dependency" >/dev/null 2>&1; then
      printf '%s\n' "development_system.release_dependency_unavailable dependency=$dependency" >&2
      exit 1
    fi
  done

  release_name="development-system-v${version}"
  archive_name="${release_name}-linux-x86_64.tar.gz"
  release_base_url="${DEVELOPMENT_SYSTEM_RELEASE_BASE_URL:-https://github.com/jwilger/ai-plugins/releases/download/$release_name}"
  download_dir="$(mktemp -d "$version_dir/.${host}.download.XXXXXX")"
  archive_path="$download_dir/$archive_name"
  checksum_path="$archive_path.sha256"
  curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
    --output "$archive_path" "$release_base_url/$archive_name" || {
    printf '%s\n' "development_system.release_download_failed asset=$archive_name" >&2
    exit 1
  }
  curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
    --output "$checksum_path" "$release_base_url/$archive_name.sha256" || {
    printf '%s\n' "development_system.release_download_failed asset=$archive_name.sha256" >&2
    exit 1
  }
  expected_checksum="$(sed -nE "s/^([0-9a-fA-F]{64})[[:space:]]+\\*?${archive_name//./\\.}$/\\1/p" "$checksum_path")"
  actual_checksum="$(sha256sum "$archive_path" | cut -d ' ' -f 1)"
  if [[ -z "$expected_checksum" ]] || [[ "${expected_checksum,,}" != "$actual_checksum" ]]; then
    printf '%s\n' "development_system.release_checksum_mismatch asset=$archive_name" >&2
    exit 1
  fi

  archive_root="${release_name}-linux-x86_64"
  expected_layout="$(printf '%s\n' \
    "$archive_root/" \
    "$archive_root/LICENSE" \
    "$archive_root/SOURCE" \
    "$archive_root/development-discipline-mcp" \
    "$archive_root/tiber")"
  actual_layout="$(tar -tzf "$archive_path" 2>/dev/null | LC_ALL=C sort)" || {
    printf '%s\n' "development_system.release_archive_invalid asset=$archive_name" >&2
    exit 1
  }
  if [[ "$actual_layout" != "$expected_layout" ]]; then
    printf '%s\n' "development_system.release_archive_invalid asset=$archive_name" >&2
    exit 1
  fi
  tar -xzf "$archive_path" -C "$download_dir" --no-same-owner --no-same-permissions || {
    printf '%s\n' "development_system.release_archive_invalid asset=$archive_name" >&2
    exit 1
  }
  for binary in tiber development-discipline-mcp; do
    if [[ ! -f "$download_dir/$archive_root/$binary" ]] || [[ -L "$download_dir/$archive_root/$binary" ]]; then
      printf '%s\n' "development_system.release_archive_invalid asset=$archive_name" >&2
      exit 1
    fi
    install -m 0755 "$download_dir/$archive_root/$binary" "$staged_dir/$binary"
  done
  for metadata in LICENSE SOURCE; do
    if [[ ! -f "$download_dir/$archive_root/$metadata" ]] || [[ -L "$download_dir/$archive_root/$metadata" ]]; then
      printf '%s\n' "development_system.release_archive_invalid asset=$archive_name" >&2
      exit 1
    fi
  done
  install -m 0644 "$download_dir/$archive_root/LICENSE" "$staged_dir/LICENSE"
  install -m 0644 "$download_dir/$archive_root/SOURCE" "$staged_dir/SOURCE"
fi

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
