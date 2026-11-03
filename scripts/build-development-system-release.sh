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

version="$(jq -er '.version' "$plugin_root/plugin.json")"
marketplace_version="$(jq -er '.plugins[] | select(.name == "development-system") | .version' "$root/.agents/plugins/marketplace.json")"
if [[ "$version" != "$marketplace_version" ]]; then
  printf '%s\n' "development_system.release_version_mismatch plugin=$version marketplace=$marketplace_version" >&2
  exit 1
fi

build_root="$(mktemp -d "${TMPDIR:-/tmp}/development-system-release.XXXXXX")"
cleanup() {
  rm -rf -- "$build_root"
}
trap cleanup EXIT

(
  cd "$plugin_root/components/development-discipline/rust"
  CARGO_TARGET_DIR="$build_root/target" cargo build --locked --release --target "$target" --bin development-discipline-mcp
)
install -m 0755 "$build_root/target/$target/release/development-discipline-mcp" "$build_root/development-discipline-mcp"

mkdir -p "$build_root/home" "$build_root/codex-home"
for binary in development-discipline-mcp; do
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

release_name="development-system-v${version}"
bundle_root="$build_root/$release_name-$platform"
mkdir -p "$bundle_root" "$output_dir"
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

source_root="$build_root/$release_name-source"
mkdir -p "$source_root/plugins/development-system/components/development-discipline" "$source_root/plugins/tiber"
tar -C "$root" --exclude=target --exclude=.plugin-eval -cf - \
  plugins/development-system/components/development-discipline/rust \
  plugins/tiber/rust plugins/development-system/plugin.json plugins/tiber/plugin.json |
  tar -C "$source_root" -xf -
install -m 0644 "$plugin_root/LICENSE" "$source_root/LICENSE"
printf '%s\n' "$version" >"$source_root/.plugin-version"
source_archive="$release_name-source.tar.gz"
tar --sort=name --mtime="@$source_date_epoch" --owner=0 --group=0 --numeric-owner \
  -C "$build_root" -czf "$output_dir/$source_archive" "$(basename "$source_root")"
(cd "$output_dir" && sha256sum "$source_archive" >"$source_archive.sha256")
