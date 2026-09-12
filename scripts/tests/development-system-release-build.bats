#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$BATS_TEST_TMPDIR"
  FAKE_BIN="$TMPROOT/fake-bin"
  mkdir -p "$FAKE_BIN"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'binary=""' \
    'target=""' \
    'while [[ $# -gt 0 ]]; do' \
    '  case "$1" in' \
    '    --bin) binary=$2; shift 2 ;;' \
    '    --target) target=$2; shift 2 ;;' \
    '    *) shift ;;' \
    '  esac' \
    'done' \
    'printf "%s|%s|%s\n" "$PWD" "$binary" "$target" >>"$CARGO_LOG"' \
    'mkdir -p "$CARGO_TARGET_DIR/$target/release"' \
    'printf "%s\n" "#!/bin/sh" "${START_COMMAND:-exit 0}" >"$CARGO_TARGET_DIR/$target/release/$binary"' \
    'if [[ "${NIX_REFERENCE:-}" == "$binary" ]]; then printf "%s\n" "/nix/store/example" >>"$CARGO_TARGET_DIR/$target/release/$binary"; fi' \
    'chmod +x "$CARGO_TARGET_DIR/$target/release/$binary"' >"$FAKE_BIN/cargo"
  printf '%s\n' '#!/bin/sh' 'if [ "${STATIC_FAILURE:-}" = 1 ]; then echo "$2: ELF 64-bit LSB pie executable, dynamically linked"; else echo "$2: ELF 64-bit LSB executable, statically linked"; fi' >"$FAKE_BIN/file"
  printf '%s\n' \
    '#!/bin/sh' \
    'if [ "${INTERPRETER_FAILURE:-}" = 1 ] && [ "$1" = -l ]; then echo "Requesting program interpreter"; fi' \
    'if [ "${NEEDED_FAILURE:-}" = 1 ] && [ "$1" = -d ]; then echo "(NEEDED) Shared library"; fi' >"$FAKE_BIN/readelf"
  chmod +x "$FAKE_BIN/cargo" "$FAKE_BIN/file" "$FAKE_BIN/readelf"
}

@test "release builder rejects a dynamically linked binary" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" STATIC_FAILURE=1 \
    "$ROOT/scripts/build-development-system-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_binary_not_static"* ]]
}

@test "release builder rejects an ELF interpreter" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" INTERPRETER_FAILURE=1 \
    "$ROOT/scripts/build-development-system-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_binary_has_interpreter"* ]]
}

@test "release builder rejects dynamic-library dependencies" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" NEEDED_FAILURE=1 \
    "$ROOT/scripts/build-development-system-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_binary_has_dynamic_dependency"* ]]
}

@test "release builder rejects Nix store runtime references" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" NIX_REFERENCE=tiber \
    "$ROOT/scripts/build-development-system-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_binary_has_nix_reference"* ]]
}

@test "release builder rejects a binary that cannot start in a minimal environment" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" START_COMMAND='exit 23' \
    "$ROOT/scripts/build-development-system-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
}

@test "release builder packages two inspected static binaries with license and exact source tag" {
  local output_dir="$TMPROOT/dist"
  local cargo_log="$TMPROOT/cargo.log"
  local version
  local archive
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  archive="$output_dir/development-system-v${version}-linux-x86_64.tar.gz"

  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$cargo_log" \
    "$ROOT/scripts/build-development-system-release.sh" "$output_dir"

  [ "$status" -eq 0 ]
  [ -f "$archive" ]
  [ -f "$archive.sha256" ]
  (cd "$output_dir" && sha256sum -c "$(basename "$archive").sha256")
  tar -tzf "$archive" | LC_ALL=C sort >"$TMPROOT/layout"
  diff -u <(printf '%s\n' \
    "development-system-v${version}-linux-x86_64/" \
    "development-system-v${version}-linux-x86_64/LICENSE" \
    "development-system-v${version}-linux-x86_64/SOURCE" \
    "development-system-v${version}-linux-x86_64/development-discipline-mcp" \
    "development-system-v${version}-linux-x86_64/tiber") "$TMPROOT/layout"
  tar -xOf "$archive" "development-system-v${version}-linux-x86_64/SOURCE" |
    grep -Fx "source=https://github.com/jwilger/ai-plugins/tree/development-system-v${version}"
  [ "$(wc -l <"$cargo_log")" -eq 2 ]
  grep -F '|tiber|x86_64-unknown-linux-musl' "$cargo_log"
  grep -F '|development-discipline-mcp|x86_64-unknown-linux-musl' "$cargo_log"
}
