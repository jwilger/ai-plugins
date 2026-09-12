#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
plugin_root="$root/plugins/development-system"
output_dir="${1:-$root/dist}"
target=x86_64-unknown-linux-musl
platform=linux-x86_64

for dependency in cargo file git jq readelf sha256sum tar; do
  command -v "$dependency" >/dev/null 2>&1 || {
    printf '%s\n' "development_system.release_build_dependency_unavailable dependency=$dependency" >&2
    exit 1
  }
done

version="$(jq -er '.version' "$plugin_root/.codex-plugin/plugin.json")"
marketplace_version="$(jq -er '.plugins[] | select(.name == "development-system") | .version' "$root/.agents/plugins/marketplace.json")"
if [[ "$version" != "$marketplace_version" ]]; then
  printf '%s\n' "development_system.release_version_mismatch plugin=$version marketplace=$marketplace_version" >&2
  exit 1
fi

tiber_toolchain="$(sed -nE 's/^channel = "([^"]+)"$/\1/p' "$plugin_root/components/tiber/rust/rust-toolchain.toml")"
discipline_toolchain="$(sed -nE 's/^channel = "([^"]+)"$/\1/p' "$plugin_root/components/development-discipline/rust/rust-toolchain.toml")"
if [[ -z "$tiber_toolchain" ]] || [[ "$tiber_toolchain" != "$discipline_toolchain" ]]; then
  printf '%s\n' "development_system.release_toolchain_mismatch tiber=$tiber_toolchain development_discipline=$discipline_toolchain" >&2
  exit 1
fi

build_root="$(mktemp -d "${TMPDIR:-/tmp}/development-system-release.XXXXXX")"
cleanup() {
  rm -rf -- "$build_root"
}
trap cleanup EXIT

build_component() {
  local component=$1
  local binary=$2
  local manifest_dir="$plugin_root/components/$component/rust"
  local target_dir="$build_root/target/$component"
  (
    cd "$manifest_dir"
    CARGO_TARGET_DIR="$target_dir" cargo build --locked --release --target "$target" --bin "$binary"
  )
  install -m 0755 "$target_dir/$target/release/$binary" "$build_root/$binary"
}

build_component tiber tiber
build_component development-discipline development-discipline-mcp

mkdir -p "$build_root/home" "$build_root/codex-home"
for binary in tiber development-discipline-mcp; do
  binary_path="$build_root/$binary"
  file "$binary_path" | grep -Eq 'static(-pie|ally) linked' || {
    printf '%s\n' "development_system.release_binary_not_static binary=$binary" >&2
    exit 1
  }
  if readelf -l "$binary_path" | grep -Fq 'Requesting program interpreter'; then
    printf '%s\n' "development_system.release_binary_has_interpreter binary=$binary" >&2
    exit 1
  fi
  if readelf -d "$binary_path" 2>/dev/null | grep -Fq '(NEEDED)'; then
    printf '%s\n' "development_system.release_binary_has_dynamic_dependency binary=$binary" >&2
    exit 1
  fi
  if LC_ALL=C grep -aFq '/nix/store/' "$binary_path"; then
    printf '%s\n' "development_system.release_binary_has_nix_reference binary=$binary" >&2
    exit 1
  fi
done

initialize_request='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"release-builder","version":"0.0.0"}}}'
initialize_binary() {
  local binary=$1
  local expected_name=$2
  shift 2
  printf '%s\n' "$initialize_request" |
    env -i PATH=/usr/bin:/bin HOME="$build_root/home" CODEX_HOME="$build_root/codex-home" \
      "$build_root/$binary" "$@" |
    grep -Fq "\"name\":\"$expected_name\""
}
initialize_binary development-discipline-mcp development-discipline --service plugin-advisory
initialize_binary tiber tiber mcp stdio

release_name="development-system-v${version}"
bundle_root="$build_root/$release_name-$platform"
mkdir -p "$bundle_root" "$output_dir"
install -m 0755 "$build_root/tiber" "$bundle_root/tiber"
install -m 0755 "$build_root/development-discipline-mcp" "$bundle_root/development-discipline-mcp"
install -m 0644 "$plugin_root/LICENSE" "$bundle_root/LICENSE"
printf '%s\n' \
  "source=https://github.com/jwilger/ai-plugins/tree/$release_name" \
  "commit=$(git -C "$root" rev-parse HEAD)" >"$bundle_root/SOURCE"

archive_name="$release_name-$platform.tar.gz"
archive_path="$output_dir/$archive_name"
source_date_epoch="${SOURCE_DATE_EPOCH:-$(git -C "$root" show -s --format=%ct HEAD)}"
tar --sort=name --mtime="@$source_date_epoch" --owner=0 --group=0 --numeric-owner \
  -C "$build_root" -czf "$archive_path" "$(basename "$bundle_root")"
(
  cd "$output_dir"
  sha256sum "$archive_name" >"$archive_name.sha256"
)

printf '%s\n' "development_system.release_built archive=$archive_path checksum=$archive_path.sha256"
