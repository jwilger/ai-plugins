#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
export RUSTUP_HOME="${RUSTUP_HOME:-$root/.dependencies/rustup}"
export CARGO_HOME="${CARGO_HOME:-$root/.dependencies/cargo}"
export PATH="$CARGO_HOME/bin:$PATH"

toolchain="${TASKBRANCH_RELEASE_TOOLCHAIN:-stable}"
manifest="$root/plugins/taskbranch/rust/Cargo.toml"
targets=(
  aarch64-unknown-linux-gnu
  x86_64-unknown-linux-gnu
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

build_target() {
  local target="$1"
  cargo zigbuild \
    --release \
    --manifest-path "$manifest" \
    --bin taskbranch \
    --target "$target"
}

copy_binary() {
  local source="$1" target="$2"
  local destination="$root/plugins/taskbranch/dist/$target/taskbranch"
  mkdir -p "$(dirname "$destination")"
  cp "$source" "$destination"
  chmod 0755 "$destination"
  echo "built $destination"
}

build_target x86_64-unknown-linux-gnu
build_target aarch64-unknown-linux-gnu
build_target universal2-apple-darwin

target_dir="$(
  cargo metadata \
    --manifest-path "$manifest" \
    --format-version 1 \
    --no-deps |
    jq -r .target_directory
)"

copy_binary "$target_dir/x86_64-unknown-linux-gnu/release/taskbranch" x86_64-unknown-linux-gnu
copy_binary "$target_dir/aarch64-unknown-linux-gnu/release/taskbranch" aarch64-unknown-linux-gnu
copy_binary "$target_dir/universal2-apple-darwin/release/taskbranch" x86_64-apple-darwin
copy_binary "$target_dir/universal2-apple-darwin/release/taskbranch" aarch64-apple-darwin
