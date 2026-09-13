#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$BATS_TEST_TMPDIR"
  VERSION="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  PLATFORM=linux-x86_64
  ARCHIVE="development-system-v${VERSION}-${PLATFORM}.tar.gz"
  RELEASE_DIR="$TMPROOT/release"
  PAYLOAD_DIR="$TMPROOT/payload/development-system-v${VERSION}-${PLATFORM}"
  mkdir -p "$RELEASE_DIR" "$PAYLOAD_DIR"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$PAYLOAD_DIR/tiber"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$PAYLOAD_DIR/development-discipline-mcp"
  chmod +x "$PAYLOAD_DIR/tiber" "$PAYLOAD_DIR/development-discipline-mcp"
  printf '%s\n' 'AGPL-3.0-or-later' >"$PAYLOAD_DIR/LICENSE"
  printf '%s\n' "source=https://github.com/jwilger/ai-plugins/tree/development-system-v${VERSION}" >"$PAYLOAD_DIR/SOURCE"
  tar -C "$TMPROOT/payload" -czf "$RELEASE_DIR/$ARCHIVE" "development-system-v${VERSION}-${PLATFORM}"
  (cd "$RELEASE_DIR" && sha256sum "$ARCHIVE" >"$ARCHIVE.sha256")

  FAKE_BIN="$TMPROOT/fake-bin"
  CURL_LOG="$TMPROOT/curl.log"
  mkdir -p "$FAKE_BIN"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'output=""' \
    'url=""' \
    'while [[ $# -gt 0 ]]; do' \
    '  case "$1" in' \
    '    --output) output=$2; shift 2 ;;' \
    '    --*) shift ;;' \
    '    *) url=$1; shift ;;' \
    '  esac' \
    'done' \
    'printf "%s\n" "$url" >>"$CURL_LOG"' \
    'case "$url" in' \
    '  *.sha256) cp "$RELEASE_DIR/$ARCHIVE.sha256" "$output" ;;' \
    '  *) cp "$RELEASE_DIR/$ARCHIVE" "$output" ;;' \
    'esac' >"$FAKE_BIN/curl"
  chmod +x "$FAKE_BIN/curl"
}

install_release() {
  env \
    PATH="$FAKE_BIN:$PATH" \
    CURL_LOG="$CURL_LOG" \
    RELEASE_DIR="$RELEASE_DIR" \
    ARCHIVE="$ARCHIVE" \
    XDG_DATA_HOME="$TMPROOT/xdg-data" \
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" "$@"
}

@test "default installation needs no Cargo and selects the exact versioned GitHub release" {
  printf '%s\n' '#!/bin/sh' 'exit 99' >"$FAKE_BIN/cargo"
  chmod +x "$FAKE_BIN/cargo"

  run install_release

  [ "$status" -eq 0 ]
  grep -Fx "https://github.com/jwilger/ai-plugins/releases/download/development-system-v${VERSION}/${ARCHIVE}" "$CURL_LOG"
  grep -Fx "https://github.com/jwilger/ai-plugins/releases/download/development-system-v${VERSION}/${ARCHIVE}.sha256" "$CURL_LOG"
  [ -x "$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64/tiber" ]
  [ -x "$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64/development-discipline-mcp" ]
  [ "$(<"$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64/.plugin-version")" = "$VERSION" ]
}

@test "session start repairs stale binaries and leaves matching binaries untouched" {
  install_release
  local installation="$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64"
  printf '%s\n' stale >"$installation/.plugin-version"
  : >"$CURL_LOG"

  run env \
    PATH="$FAKE_BIN:$PATH" \
    CURL_LOG="$CURL_LOG" \
    RELEASE_DIR="$RELEASE_DIR" \
    ARCHIVE="$ARCHIVE" \
    XDG_DATA_HOME="$TMPROOT/xdg-data" \
    "$ROOT/plugins/development-system/bin/development-system" session-start \
    --project "$TMPROOT" --format json

  [ "$status" -eq 0 ]
  [ "$(<"$installation/.plugin-version")" = "$VERSION" ]
  [ "$(wc -l <"$CURL_LOG")" -eq 2 ]
  : >"$CURL_LOG"

  run env \
    PATH="$FAKE_BIN:$PATH" \
    CURL_LOG="$CURL_LOG" \
    RELEASE_DIR="$RELEASE_DIR" \
    ARCHIVE="$ARCHIVE" \
    XDG_DATA_HOME="$TMPROOT/xdg-data" \
    "$ROOT/plugins/development-system/bin/development-system" session-start \
    --project "$TMPROOT" --format json

  [ "$status" -eq 0 ]
  [ ! -s "$CURL_LOG" ]
}

@test "automatic installation builds on a host without a prebuilt release" {
  local fake_target="$TMPROOT/auto-source-target"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'if [[ "$1" == "--version" ]]; then echo "cargo fake"; exit 0; fi' \
    'binary=""' \
    'while [[ $# -gt 0 ]]; do' \
    '  if [[ "$1" == "--bin" ]]; then binary="$2"; shift 2; continue; fi' \
    '  shift' \
    'done' \
    'mkdir -p "$CARGO_TARGET_DIR/release"' \
    'printf "%s\n" "#!/bin/sh" "exit 0" >"$CARGO_TARGET_DIR/release/$binary"' \
    'chmod +x "$CARGO_TARGET_DIR/release/$binary"' >"$FAKE_BIN/cargo"
  chmod +x "$FAKE_BIN/cargo"

  run env \
    PATH="$FAKE_BIN:$PATH" \
    CARGO_TARGET_DIR="$fake_target" \
    XDG_DATA_HOME="$TMPROOT/auto-source-xdg" \
    DEVELOPMENT_SYSTEM_HOST_OVERRIDE=darwin-aarch64 \
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" --auto

  [ "$status" -eq 0 ]
  local installation="$TMPROOT/auto-source-xdg/ai-plugins/development-system/$VERSION/darwin-aarch64"
  [ -x "$installation/tiber" ]
  [ -x "$installation/development-discipline-mcp" ]
  [ "$(<"$installation/.plugin-version")" = "$VERSION" ]
}

@test "checksum mismatch preserves the prior installation" {
  install_release
  local link="$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64"
  local previous
  previous="$(readlink "$link")"
  printf '%064d  %s\n' 0 "$ARCHIVE" >"$RELEASE_DIR/$ARCHIVE.sha256"

  run install_release

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_checksum_mismatch"* ]]
  [ "$(readlink "$link")" = "$previous" ]
}

@test "malformed archives preserve the prior installation" {
  install_release
  local link="$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64"
  local previous
  previous="$(readlink "$link")"
  printf 'unexpected\n' >"$TMPROOT/payload/unexpected"
  tar -C "$TMPROOT/payload" -czf "$RELEASE_DIR/$ARCHIVE" unexpected
  (cd "$RELEASE_DIR" && sha256sum "$ARCHIVE" >"$ARCHIVE.sha256")

  run install_release

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_archive_invalid"* ]]
  [ "$(readlink "$link")" = "$previous" ]
}

@test "interrupted download preserves the prior installation" {
  install_release
  local link="$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64"
  local previous
  previous="$(readlink "$link")"
  printf '%s\n' '#!/bin/sh' 'exit 18' >"$FAKE_BIN/curl"
  chmod +x "$FAKE_BIN/curl"

  run install_release

  [ "$status" -ne 0 ]
  [ "$(readlink "$link")" = "$previous" ]
}

@test "download mode rejects unsupported hosts with source-build remediation" {
  run env \
    PATH="$FAKE_BIN:$PATH" \
    XDG_DATA_HOME="$TMPROOT/xdg-data" \
    DEVELOPMENT_SYSTEM_HOST_OVERRIDE=darwin-aarch64 \
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh"

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.release_host_unsupported"* ]]
  [[ "$output" == *"--from-source"* ]]
}

@test "concurrent verified installs serialize publication and reruns recover cleanly" {
  run env ROOT="$ROOT" FAKE_BIN="$FAKE_BIN" CURL_LOG="$CURL_LOG" RELEASE_DIR="$RELEASE_DIR" ARCHIVE="$ARCHIVE" TMPROOT="$TMPROOT" bash -c '
    PATH="$FAKE_BIN:$PATH" XDG_DATA_HOME="$TMPROOT/xdg-data" "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" >"$TMPROOT/one.log" 2>&1 & one=$!
    PATH="$FAKE_BIN:$PATH" XDG_DATA_HOME="$TMPROOT/xdg-data" "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" >"$TMPROOT/two.log" 2>&1 & two=$!
    wait "$one" && wait "$two"
  '

  [ "$status" -eq 0 ]
  local link="$TMPROOT/xdg-data/ai-plugins/development-system/$VERSION/linux-x86_64"
  [ -L "$link" ]
  [ -x "$link/tiber" ]
  [ "$(find "$(dirname "$link")" -maxdepth 1 -name '.linux-x86_64.staging.*' | wc -l)" -eq 1 ]
}
