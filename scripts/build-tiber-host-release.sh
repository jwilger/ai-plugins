#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64 | Linux-arm64) target="aarch64-unknown-linux-gnu" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  Darwin-arm64 | Darwin-aarch64) target="aarch64-apple-darwin" ;;
  *)
    echo "tiber.unsupported_release_host os=$(uname -s) arch=$(uname -m)" >&2
    exit 1
    ;;
esac

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
destination="$root/plugins/tiber/dist/$target/tiber"
mkdir -p "$(dirname "$destination")"
cp "$target_dir/release/tiber" "$destination"
chmod 0755 "$destination"
echo "built $destination"
