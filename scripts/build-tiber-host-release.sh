#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

source "$root/plugins/tiber/scripts/detect-target.sh"
target="$(detect_tiber_target)" || {
  echo "tiber.unsupported_release_host os=$(uname -s) arch=$(uname -m)" >&2
  exit 1
}

cargo build \
  --release \
  --manifest-path "$root/plugins/tiber/rust/Cargo.toml" \
  --bin tiber

target_dir="$(
  cargo metadata \
    --manifest-path "$root/plugins/tiber/rust/Cargo.toml" \
    --format-version 1 \
    --no-deps |
    jq -r .target_directory
)"
destination="$root/.dependencies/tiber-host-release/$target/tiber"
mkdir -p "$(dirname "$destination")"
cp "$target_dir/release/tiber" "$destination"
chmod 0755 "$destination"
echo "built dev-only host binary $destination"
echo "bundled release artifacts are built by scripts/build-tiber-release-all.sh"
