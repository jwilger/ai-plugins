#!/usr/bin/env bash

# Sets source_root to either a complete developer checkout or a verified release
# source closure. Installed companion caches are never used as dependencies.
plugin_prepare_source() {
  local plugin_name=$1 plugin_root=$2 version=$3 version_dir=$4 release_base_url=$5
  local checkout_root archive_name checksum expected actual member
  checkout_root="$(cd "$plugin_root/../.." && pwd -P)"
  if [[ -e "$checkout_root/.git" ]] &&
    [[ -f "$checkout_root/.agents/plugins/marketplace.json" ]] &&
    [[ -f "$checkout_root/plugins/tiber/rust/crates/tiber-git/Cargo.toml" ]] &&
    [[ -f "$checkout_root/plugins/development-system/components/development-discipline/rust/Cargo.toml" ]]; then
    source_root="$checkout_root"
    return
  fi
  for member in curl tar sha256sum; do
    command -v "$member" >/dev/null 2>&1 || {
      printf '%s\n' "$plugin_name.source_dependency_unavailable dependency=$member" >&2
      return 1
    }
  done
  download_dir="$(mktemp -d "$version_dir/.source.download.XXXXXX")"
  archive_name="$plugin_name-v$version-source.tar.gz"
  curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
    --output "$download_dir/$archive_name" "$release_base_url/$archive_name" || return 1
  curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
    --output "$download_dir/$archive_name.sha256" "$release_base_url/$archive_name.sha256" || return 1
  checksum="$download_dir/$archive_name.sha256"
  expected="$(sed -nE "s/^([0-9a-fA-F]{64})[[:space:]]+\\*?${archive_name//./\\.}$/\\1/p" "$checksum")"
  actual="$(sha256sum "$download_dir/$archive_name" | cut -d ' ' -f 1)"
  if [[ -z "$expected" ]] || [[ "${expected,,}" != "$actual" ]]; then
    printf '%s\n' "$plugin_name.source_checksum_mismatch" >&2
    return 1
  fi
  tar -tzf "$download_dir/$archive_name" >"$download_dir/members" || return 1
  while IFS= read -r member; do
    case "$member" in
      "$plugin_name-v$version-source/"*) ;;
      *) return 1 ;;
    esac
    case "/${member%/}/" in
      */../* | */./* | *//* ) return 1 ;;
    esac
  done <"$download_dir/members"
  # Reject links and special nodes before extraction, including hard links.
  tar -tvzf "$download_dir/$archive_name" | awk 'substr($0,1,1) !~ /[-d]/ {bad=1} END {exit bad}' || return 1
  tar -xzf "$download_dir/$archive_name" -C "$download_dir" --no-same-owner --no-same-permissions || return 1
  source_root="$download_dir/$plugin_name-v$version-source"
  [[ -f "$source_root/.plugin-version" ]] && [[ "$(<"$source_root/.plugin-version")" == "$version" ]] || return 1
}
