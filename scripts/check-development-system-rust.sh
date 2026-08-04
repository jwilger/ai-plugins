#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd -P)"
components_root="$repo_root/plugins/development-system/components"
cd "$repo_root"

mapfile -t manifests < <(
  find "$components_root" -mindepth 3 -maxdepth 3 -path '*/rust/Cargo.toml' -print \
    | LC_ALL=C sort
)

if [ "${#manifests[@]}" -eq 0 ]; then
  printf 'No development-system Rust component manifests found.\n' >&2
  exit 1
fi

if [ "${1:-}" = "--list-manifests" ]; then
  printf '%s\n' "${manifests[@]#$repo_root/}"
  exit 0
fi

if [ "$#" -ne 0 ]; then
  printf 'Usage: %s [--list-manifests]\n' "$0" >&2
  exit 2
fi

for manifest in "${manifests[@]}"; do
  component="${manifest#"$components_root"/}"
  component="${component%/rust/Cargo.toml}"
  manifest_path="${manifest#"$repo_root"/}"
  target_dir="${CARGO_TARGET_DIR:-$repo_root/.dependencies/cargo-target/development-system/$component}"

  CARGO_TARGET_DIR="$target_dir" cargo fmt --manifest-path "$manifest_path" --all --check
  CARGO_TARGET_DIR="$target_dir" cargo clippy --manifest-path "$manifest_path" --all-targets -- -D warnings
  CARGO_TARGET_DIR="$target_dir" cargo test --manifest-path "$manifest_path" -- --test-threads=1
done
