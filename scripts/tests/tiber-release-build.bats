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
    'if [[ -n "${START_COMMAND:-}" ]]; then' \
    '  printf "%s\n" "#!/bin/sh" "$START_COMMAND" >"$CARGO_TARGET_DIR/$target/release/$binary"' \
    'else' \
    '  printf "%s\n" "#!/bin/sh" "read request" "case \"\${0##*/}\" in" "  tiber) echo '\''{\"name\":\"tiber\"}'\'' ;;" "  *) echo '\''{\"name\":\"development-discipline\"}'\'' ;;" "esac" >"$CARGO_TARGET_DIR/$target/release/$binary"' \
    'fi' \
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
    "$ROOT/scripts/build-tiber-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"tiber.release_binary_not_static"* ]]
}

@test "release builder rejects an ELF interpreter" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" INTERPRETER_FAILURE=1 \
    "$ROOT/scripts/build-tiber-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"tiber.release_binary_has_interpreter"* ]]
}

@test "release builder rejects dynamic-library dependencies" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" NEEDED_FAILURE=1 \
    "$ROOT/scripts/build-tiber-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"tiber.release_binary_has_dynamic_dependency"* ]]
}

@test "release builder rejects Nix store runtime references" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" NIX_REFERENCE=tiber \
    "$ROOT/scripts/build-tiber-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
  [[ "$output" == *"tiber.release_binary_has_nix_reference"* ]]
}

@test "release builder rejects a binary that cannot start in a minimal environment" {
  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$TMPROOT/cargo.log" START_COMMAND='exit 23' \
    "$ROOT/scripts/build-tiber-release.sh" "$TMPROOT/dist"

  [ "$status" -ne 0 ]
}

@test "release builder packages its inspected static binary with license and exact source tag" {
  local output_dir="$TMPROOT/dist"
  local cargo_log="$TMPROOT/cargo.log"
  local version
  local archive
  version="$(jq -r '.version' "$ROOT/plugins/tiber/plugin.json")"
  archive="$output_dir/tiber-v${version}-linux-x86_64.tar.gz"

  run env PATH="$FAKE_BIN:$PATH" CARGO_LOG="$cargo_log" \
    "$ROOT/scripts/build-tiber-release.sh" "$output_dir"

  [ "$status" -eq 0 ]
  [ -f "$archive" ]
  [ -f "$archive.sha256" ]
  local source_archive="$output_dir/tiber-v${version}-source.tar.gz"
  [ -f "$source_archive" ]
  (cd "$output_dir" && sha256sum -c "$(basename "$source_archive").sha256")
  tar -tzf "$source_archive" | grep -F '/plugins/tiber/rust/crates/tiber-git/Cargo.toml'
  tar -tzf "$source_archive" | grep -F '/plugins/development-system/components/development-discipline/rust/Cargo.toml'
  tar -tzf "$source_archive" | grep -F '/plugins/development-system/schemas/project-config.schema.json'
  ! tar -tzf "$source_archive" | grep -E '/target/'
  mkdir -p "$TMPROOT/source-check"
  tar -xzf "$source_archive" -C "$TMPROOT/source-check"
  local closure_root="$TMPROOT/source-check/tiber-v${version}-source"
  cargo metadata --locked --no-deps --format-version 1 \
    --manifest-path "$closure_root/plugins/development-system/components/development-discipline/rust/Cargo.toml" >"$TMPROOT/discipline-metadata.json"
  cargo metadata --locked --no-deps --format-version 1 \
    --manifest-path "$closure_root/plugins/tiber/rust/Cargo.toml" >"$TMPROOT/tiber-metadata.json"
 
  (cd "$output_dir" && sha256sum -c "$(basename "$archive").sha256")
  tar -tzf "$archive" | LC_ALL=C sort >"$TMPROOT/layout"
  diff -u <(printf '%s\n' \
    "tiber-v${version}-linux-x86_64/" \
    "tiber-v${version}-linux-x86_64/LICENSE" \
    "tiber-v${version}-linux-x86_64/SOURCE" \
    "tiber-v${version}-linux-x86_64/tiber") "$TMPROOT/layout"
  tar -xOf "$archive" "tiber-v${version}-linux-x86_64/SOURCE" |
    grep -Fx "source=https://github.com/jwilger/ai-plugins/tree/tiber-v${version}"
  [ "$(wc -l <"$cargo_log")" -eq 1 ]
  grep -F '|tiber|x86_64-unknown-linux-musl' "$cargo_log"
}
