#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  DETECTOR="$ROOT/plugins/development-discipline/scripts/detect-target.sh"
  COMPLETE_CHECK="$ROOT/scripts/check-development-discipline-release-complete.sh"
  FAKE_BIN="$BATS_TEST_TMPDIR/fake-bin"
  mkdir -p "$FAKE_BIN"
  printf '%s\n' \
    '#!/bin/sh' \
    'case "$1" in' \
    '  -s) printf "%s\n" "$FAKE_UNAME_S" ;;' \
    '  -m) printf "%s\n" "$FAKE_UNAME_M" ;;' \
    '  *) exit 1 ;;' \
    'esac' >"$FAKE_BIN/uname"
  chmod +x "$FAKE_BIN/uname"
}

detect_target() {
  env \
    FAKE_UNAME_S="$1" \
    FAKE_UNAME_M="$2" \
    bash -c 'source "$1"; detect_development_discipline_target "$2"' _ "$DETECTOR" "$FAKE_BIN/uname"
}

@test "development-discipline release includes every supported target" {
  run bash "$COMPLETE_CHECK"

  [ "$status" -eq 0 ]
}

@test "development-discipline release artifacts embed current source provenance" {
  local expected
  local actual
  local binary_path

  expected="$(
    cd "$ROOT/plugins/development-discipline/rust"
    sha256sum Cargo.toml Cargo.lock rust-toolchain.toml src/main.rs | sha256sum | awk '{ print $1 }'
  )"
  actual="$(jq -r '.source_fingerprint' "$ROOT/plugins/development-discipline/release-binaries.json")"
  [ "$actual" = "$expected" ]

  while IFS= read -r binary_path; do
    grep -aFq "$expected" "$ROOT/plugins/development-discipline/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-discipline/release-binaries.json")
}

@test "development-discipline release build pins and fingerprints its Rust toolchain" {
  local toolchain="$ROOT/plugins/development-discipline/rust/rust-toolchain.toml"

  run rg -n '^channel = "[0-9]+\.[0-9]+\.[0-9]+"$' "$toolchain"
  [ "$status" -eq 0 ]

  run rg -n 'rust-toolchain.toml' "$ROOT/scripts/build-development-discipline-release-all.sh"
  [ "$status" -eq 0 ]

  run rg -n 'rustup run "\$toolchain" cargo zigbuild' "$ROOT/scripts/build-development-discipline-release-all.sh"
  [ "$status" -eq 0 ]

  run rg -n 'rust-toolchain.toml' "$ROOT/scripts/check-development-discipline-release-complete.sh"
  [ "$status" -eq 0 ]

  run rg -n 'rustup run "\$toolchain" cargo build' "$ROOT/scripts/check-development-discipline-release-from-source.sh"
  [ "$status" -eq 0 ]
}

@test "development-discipline release artifacts match their declared architectures" {
  local plugin_root="$ROOT/plugins/development-discipline"

  run file "$plugin_root/dist/x86_64-unknown-linux-musl/development-discipline-mcp"
  [ "$status" -eq 0 ]
  [[ "$output" == *"ELF 64-bit"* ]]
  [[ "$output" == *"x86-64"* ]]
  [[ "$output" == *"statically linked"* ]]

  run file "$plugin_root/dist/aarch64-unknown-linux-musl/development-discipline-mcp"
  [ "$status" -eq 0 ]
  [[ "$output" == *"ELF 64-bit"* ]]
  [[ "$output" == *"ARM aarch64"* ]]
  [[ "$output" == *"statically linked"* ]]

  run file "$plugin_root/dist/x86_64-apple-darwin/development-discipline-mcp"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Mach-O universal binary"* ]]
  [[ "$output" == *"x86_64"* ]]
  [[ "$output" == *"arm64"* ]]

  run file "$plugin_root/dist/aarch64-apple-darwin/development-discipline-mcp"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Mach-O universal binary"* ]]
  [[ "$output" == *"x86_64"* ]]
  [[ "$output" == *"arm64"* ]]
}

@test "development-discipline target detector maps supported Linux and macOS hosts" {
  run detect_target Linux x86_64
  [ "$status" -eq 0 ]
  [ "$output" = "x86_64-unknown-linux-musl" ]

  run detect_target Linux aarch64
  [ "$status" -eq 0 ]
  [ "$output" = "aarch64-unknown-linux-musl" ]

  run detect_target Darwin x86_64
  [ "$status" -eq 0 ]
  [ "$output" = "x86_64-apple-darwin" ]

  run detect_target Darwin arm64
  [ "$status" -eq 0 ]
  [ "$output" = "aarch64-apple-darwin" ]
}

@test "development-discipline target detector rejects unsupported hosts" {
  run detect_target FreeBSD amd64

  [ "$status" -ne 0 ]
  [ -z "$output" ]
}
