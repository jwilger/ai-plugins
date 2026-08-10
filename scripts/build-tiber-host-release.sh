#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

source "$root/plugins/development-system/components/tiber/scripts/detect-target.sh"
target="$(detect_tiber_target)" || {
  echo "tiber.unsupported_release_host os=$(uname -s) arch=$(uname -m)" >&2
  exit 1
}

manifest="$root/plugins/development-system/components/tiber/rust/Cargo.toml"
if [ "$target" = x86_64-unknown-linux-gnu ]; then
  cargo zigbuild \
    --release \
    --manifest-path "$manifest" \
    --bin tiber \
    --target "$target"
  release_subdirectory="$target/release"
else
  cargo build \
    --release \
    --manifest-path "$manifest" \
    --bin tiber
  release_subdirectory=release
fi

target_dir="$(
  cargo metadata \
    --manifest-path "$manifest" \
    --format-version 1 \
    --no-deps |
    jq -r .target_directory
)"
destination="$root/plugins/development-system/components/tiber/dist/$target/tiber"
mkdir -p "$(dirname "$destination")"
cp "$target_dir/$release_subdirectory/tiber" "$destination"
chmod 0755 "$destination"
echo "built $destination"
