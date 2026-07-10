#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
export RUSTUP_HOME="${RUSTUP_HOME:-$root/.dependencies/rustup}"
export CARGO_HOME="${CARGO_HOME:-$root/.dependencies/cargo}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/.dependencies/cargo-target/development-discipline-release-all}"
export ZIG_GLOBAL_CACHE_DIR="${ZIG_GLOBAL_CACHE_DIR:-$root/.dependencies/zig/global-cache}"
export ZIG_LOCAL_CACHE_DIR="${ZIG_LOCAL_CACHE_DIR:-$root/.dependencies/zig/local-cache}"
export XDG_CACHE_HOME="${DEVELOPMENT_DISCIPLINE_RELEASE_XDG_CACHE_HOME:-$root/.dependencies/xdg-cache}"
export PATH="$CARGO_HOME/bin:$PATH"
mkdir -p "$ZIG_GLOBAL_CACHE_DIR" "$ZIG_LOCAL_CACHE_DIR" "$XDG_CACHE_HOME"

manifest="$root/plugins/development-discipline/rust/Cargo.toml"
plugin_root="$root/plugins/development-discipline"
toolchain_file="$plugin_root/rust/rust-toolchain.toml"
toolchain="$(awk -F'"' '/^channel = "/ { print $2; exit }' "$toolchain_file")"
if [ -z "$toolchain" ]; then
  echo "release-toolchain-channel-missing path=plugins/development-discipline/rust/rust-toolchain.toml" >&2
  exit 1
fi
binary_name="development-discipline-mcp"
source_fingerprint="$(
  cd "$plugin_root/rust"
  sha256sum Cargo.toml Cargo.lock rust-toolchain.toml src/main.rs | sha256sum | awk '{ print $1 }'
)"
export DEVELOPMENT_DISCIPLINE_SOURCE_FINGERPRINT="$source_fingerprint"
targets=(
  aarch64-unknown-linux-musl
  x86_64-unknown-linux-musl
  x86_64-apple-darwin
  aarch64-apple-darwin
)

if ! rustup toolchain list | grep -Eq "^${toolchain}(-| )"; then
  rustup toolchain install "$toolchain" --profile minimal
fi

installed_targets="$(rustup target list --installed --toolchain "$toolchain")"
for target in "${targets[@]}"; do
  if ! grep -Fxq "$target" <<<"$installed_targets"; then
    rustup target add "$target" --toolchain "$toolchain"
  fi
done

toolchain_bin="$(rustup run "$toolchain" rustc --print sysroot)/bin"
export PATH="$toolchain_bin:$PATH"

if ! command -v zig >/dev/null 2>&1; then
  echo "missing-release-tool tool=zig" >&2
  exit 1
fi
if ! rustup run "$toolchain" cargo zigbuild --help >/dev/null 2>&1; then
  echo "missing-release-tool tool=cargo-zigbuild" >&2
  exit 1
fi

build_target() {
  rustup run "$toolchain" cargo zigbuild \
    --release \
    --manifest-path "$manifest" \
    --bin "$binary_name" \
    --target "$1"
}

copy_binary() {
  local source="$1" target="$2"
  local destination="$plugin_root/dist/$target/$binary_name"
  mkdir -p "$(dirname "$destination")"
  cp "$source" "$destination"
  chmod 0755 "$destination"
  echo "built $destination"
}

build_target x86_64-unknown-linux-musl
build_target aarch64-unknown-linux-musl
build_target universal2-apple-darwin

copy_binary "$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/$binary_name" x86_64-unknown-linux-musl
copy_binary "$CARGO_TARGET_DIR/aarch64-unknown-linux-musl/release/$binary_name" aarch64-unknown-linux-musl
copy_binary "$CARGO_TARGET_DIR/universal2-apple-darwin/release/$binary_name" x86_64-apple-darwin
copy_binary "$CARGO_TARGET_DIR/universal2-apple-darwin/release/$binary_name" aarch64-apple-darwin

manifest_tmp="$(mktemp)"
checksums_tmp="$(mktemp)"
trap 'rm -f "$manifest_tmp" "$checksums_tmp"' EXIT
printf '{\n  "source_fingerprint": "%s",\n  "binaries": [\n' "$source_fingerprint" >"$manifest_tmp"
for index in "${!targets[@]}"; do
  target="${targets[$index]}"
  path="dist/$target/$binary_name"
  sha="$(sha256sum "$plugin_root/$path" | awk '{ print $1 }')"
  [ "$index" -eq 0 ] || printf ',\n' >>"$manifest_tmp"
  printf '    {\n      "target": "%s",\n      "path": "%s",\n      "sha256": "%s"\n    }' \
    "$target" "$path" "$sha" >>"$manifest_tmp"
  printf '%s  %s\n' "$sha" "$path" >>"$checksums_tmp"
done
printf '\n  ]\n}\n' >>"$manifest_tmp"
mv "$manifest_tmp" "$plugin_root/release-binaries.json"
mv "$checksums_tmp" "$plugin_root/release-binaries.sha256"
echo "wrote $plugin_root/release-binaries.json"
echo "wrote $plugin_root/release-binaries.sha256"
