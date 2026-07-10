#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
plugin_root="$root/plugins/development-discipline"
manifest="$plugin_root/release-binaries.json"
checksums="$plugin_root/release-binaries.sha256"

if [ ! -f "$manifest" ]; then
  echo "missing-release-manifest path=plugins/development-discipline/release-binaries.json" >&2
  exit 1
fi

if [ ! -f "$checksums" ]; then
  echo "missing-release-checksums path=plugins/development-discipline/release-binaries.sha256" >&2
  exit 1
fi

jq -e '.binaries | type == "array" and length > 0' "$manifest" >/dev/null

expected_source_fingerprint="$(
  cd "$plugin_root/rust"
  sha256sum Cargo.toml Cargo.lock rust-toolchain.toml src/main.rs | sha256sum | awk '{ print $1 }'
)"
manifest_source_fingerprint="$(jq -r '.source_fingerprint // empty' "$manifest")"
if [ "$manifest_source_fingerprint" != "$expected_source_fingerprint" ]; then
  echo "release-source-fingerprint-mismatch" >&2
  exit 1
fi

expected_targets="$(printf '%s\n' \
  aarch64-apple-darwin \
  aarch64-unknown-linux-musl \
  x86_64-apple-darwin \
  x86_64-unknown-linux-musl)"
actual_targets="$(jq -r '.binaries[].target' "$manifest" | sort)"
if [ "$actual_targets" != "$expected_targets" ]; then
  echo "release-binary-targets-incomplete" >&2
  diff -u <(printf '%s\n' "$expected_targets") <(printf '%s\n' "$actual_targets") >&2 || true
  exit 1
fi

manifest_paths="$(mktemp)"
checksum_paths="$(mktemp)"
trap 'rm -f "$manifest_paths" "$checksum_paths"' EXIT

jq -r '.binaries[].path' "$manifest" | sort >"$manifest_paths"
awk '{ print $2 }' "$checksums" | sort >"$checksum_paths"

if ! cmp -s "$manifest_paths" "$checksum_paths"; then
  echo "release-checksum-paths-mismatch path=plugins/development-discipline/release-binaries.sha256" >&2
  diff -u "$checksum_paths" "$manifest_paths" >&2 || true
  exit 1
fi

while IFS= read -r encoded; do
  target="$(printf '%s' "$encoded" | base64 -d | jq -r '.target')"
  path="$(printf '%s' "$encoded" | base64 -d | jq -r '.path')"
  expected_sha="$(printf '%s' "$encoded" | base64 -d | jq -r '.sha256')"
  binary="$plugin_root/$path"

  if [ -z "$target" ] || [ "$target" = "null" ]; then
    echo "release-binary-target-missing path=$path" >&2
    exit 1
  fi
  if [ ! -s "$binary" ]; then
    echo "release-binary-missing-or-empty path=plugins/development-discipline/$path" >&2
    exit 1
  fi
  if [ ! -x "$binary" ]; then
    echo "release-binary-not-executable path=plugins/development-discipline/$path" >&2
    exit 1
  fi
  if ! grep -aFq "$expected_source_fingerprint" "$binary"; then
    echo "release-binary-source-fingerprint-mismatch target=$target" >&2
    exit 1
  fi

  binary_description="$(file -b "$binary")"
  case "$target" in
    x86_64-unknown-linux-musl)
      required_format="ELF 64-bit"
      required_arch="x86-64"
      ;;
    aarch64-unknown-linux-musl)
      required_format="ELF 64-bit"
      required_arch="ARM aarch64"
      ;;
    x86_64-apple-darwin)
      required_format="Mach-O"
      required_arch="x86_64"
      ;;
    aarch64-apple-darwin)
      required_format="Mach-O"
      required_arch="arm64"
      ;;
    *)
      echo "release-binary-target-unsupported target=$target" >&2
      exit 1
      ;;
  esac
  if [[ "$binary_description" != *"$required_format"* || "$binary_description" != *"$required_arch"* ]]; then
    echo "release-binary-format-mismatch target=$target description=$binary_description" >&2
    exit 1
  fi
  case "$target" in
    *-unknown-linux-musl)
      if [[ "$binary_description" != *"statically linked"* ]]; then
        echo "release-binary-not-static target=$target description=$binary_description" >&2
        exit 1
      fi
      ;;
  esac

  actual_sha="$(sha256sum "$binary" | awk '{ print $1 }')"
  checksum_sha="$(awk -v path="$path" '$2 == path { print $1 }' "$checksums")"
  if [ "$actual_sha" != "$expected_sha" ] || [ "$actual_sha" != "$checksum_sha" ]; then
    echo "release-binary-sha-mismatch path=plugins/development-discipline/$path" >&2
    exit 1
  fi
done < <(jq -r '.binaries[] | @base64' "$manifest")
