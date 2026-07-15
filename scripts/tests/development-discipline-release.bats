#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  DETECTOR="$ROOT/plugins/development-discipline/scripts/detect-target.sh"
  COMPLETE_CHECK="$ROOT/scripts/check-development-discipline-release-complete.sh"
  PARITY_CHECK="$ROOT/scripts/check-development-discipline-release-from-source.sh"
  PARITY_NORMALIZER="$ROOT/scripts/tests/development-discipline-parity-normalize.mjs"
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

@test "development-discipline parity normalization removes runtime clock drift" {
  local source_output="$BATS_TEST_TMPDIR/source.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist.jsonl"
  local normalized_source

  printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' >"$source_output"
  printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" = "$normalized_source" ]
}

@test "development-discipline parity normalization preserves review-budget clock relationships" {
  local source_output="$BATS_TEST_TMPDIR/source-clock-relationships.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-clock-relationships.jsonl"
  local normalized_source

  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' >"$source_output"
  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"cccccccccccccccc\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"dddddddddddddddd\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":102}}}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization isolates clocks between review sessions" {
  local source_output="$BATS_TEST_TMPDIR/source-session-clocks.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-session-clocks.jsonl"
  local normalized_source

  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-two\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' >"$source_output"
  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"cccccccccccccccc\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-two\",\"review_contract_id\":\"dddddddddddddddd\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":102}}}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" = "$normalized_source" ]
}

@test "development-discipline parity normalization removes derived transition drift" {
  local source_output="$BATS_TEST_TMPDIR/source-transition.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-transition.jsonl"
  local normalized_source

  printf '%s\n' '{"jsonrpc":"2.0","id":7,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}},\"verified_clean_iterations\":[{\"iteration\":1,\"transition_id\":\"1111111111111111\"}]}}"}]}}' >"$source_output"
  printf '%s\n' '{"jsonrpc":"2.0","id":7,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}},\"verified_clean_iterations\":[{\"iteration\":1,\"transition_id\":\"2222222222222222\"}]}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" = "$normalized_source" ]
}

@test "development-discipline parity normalization preserves same-named fields outside review state" {
  local source_output="$BATS_TEST_TMPDIR/source-unrelated.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-unrelated.jsonl"
  local normalized_source

  printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}},\"unrelated\":{\"started_at_epoch_seconds\":1}}"}]}}' >"$source_output"
  printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}},\"unrelated\":{\"started_at_epoch_seconds\":2}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization preserves malformed review state" {
  local source_output="$BATS_TEST_TMPDIR/source-malformed-state.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-malformed-state.jsonl"
  local normalized_source

  printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{}}}}"}]}}' >"$source_output"
  printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{}}}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization preserves state without a canonical session ID" {
  local source_output="$BATS_TEST_TMPDIR/source-missing-session.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-missing-session.jsonl"
  local normalized_source

  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}}}"}]}}' >"$source_output"
  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"review_contract_id\":\"cccccccccccccccc\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":102}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review two\",\"review_contract_id\":\"dddddddddddddddd\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":103}}}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  [ "$output" = "$(<"$source_output")" ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" = "$(<"$dist_output")" ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization preserves contract ID relationships" {
  local source_output="$BATS_TEST_TMPDIR/source-contract-relationships.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-contract-relationships.jsonl"
  local normalized_source

  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}}}}"}]}}' >"$source_output"
  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}}}"}]}}' \
    '{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"cccccccccccccccc\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}}}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization preserves transition ID relationships" {
  local source_output="$BATS_TEST_TMPDIR/source-transition-relationships.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-transition-relationships.jsonl"
  local normalized_source

  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":7,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}},\"verified_clean_iterations\":[{\"iteration\":1,\"transition_id\":\"1111111111111111\"}]}}"}]}}' \
    '{"jsonrpc":"2.0","id":8,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"bbbbbbbbbbbbbbbb\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}},\"verified_clean_iterations\":[{\"iteration\":2,\"transition_id\":\"1111111111111111\"}]}}"}]}}' >"$source_output"
  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":7,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"cccccccccccccccc\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}},\"verified_clean_iterations\":[{\"iteration\":1,\"transition_id\":\"2222222222222222\"}]}}"}]}}' \
    '{"jsonrpc":"2.0","id":8,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"dddddddddddddddd\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":101}},\"verified_clean_iterations\":[{\"iteration\":2,\"transition_id\":\"3333333333333333\"}]}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization preserves transition IDs for zero iteration" {
  local source_output="$BATS_TEST_TMPDIR/source-zero-iteration.jsonl"
  local dist_output="$BATS_TEST_TMPDIR/dist-zero-iteration.jsonl"
  local normalized_source

  printf '%s\n' '{"jsonrpc":"2.0","id":7,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}},\"verified_clean_iterations\":[{\"iteration\":0,\"transition_id\":\"1111111111111111\"}]}}"}]}}' >"$source_output"
  printf '%s\n' '{"jsonrpc":"2.0","id":7,"result":{"content":[{"type":"text","text":"{\"state\":{\"session_id\":\"review-one\",\"review_contract_id\":\"aaaaaaaaaaaaaaaa\",\"risk_plan\":{\"review_budget\":{\"started_at_epoch_seconds\":100}},\"verified_clean_iterations\":[{\"iteration\":0,\"transition_id\":\"2222222222222222\"}]}}"}]}}' >"$dist_output"

  run node "$PARITY_NORMALIZER" "$source_output"
  [ "$status" -eq 0 ]
  normalized_source="$output"

  run node "$PARITY_NORMALIZER" "$dist_output"
  [ "$status" -eq 0 ]
  [ "$output" != "$normalized_source" ]
}

@test "development-discipline parity normalization rejects interior blank records" {
  local malformed_output="$BATS_TEST_TMPDIR/interior-blank.jsonl"

  printf '%s\n\n%s\n' \
    '{"jsonrpc":"2.0","id":1,"result":{}}' \
    '{"jsonrpc":"2.0","id":2,"result":{}}' >"$malformed_output"

  run node "$PARITY_NORMALIZER" "$malformed_output"

  [ "$status" -ne 0 ]
  [[ "$output" == *"$malformed_output"* ]]
  [[ "$output" == *"record=2"* ]]
  [[ "$output" == *"blank record"* ]]
}

@test "development-discipline parity normalization identifies invalid JSON records" {
  local malformed_output="$BATS_TEST_TMPDIR/invalid-json.jsonl"

  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":1,"result":{}}' \
    '{"jsonrpc":"2.0","id":2,"result":' >"$malformed_output"

  run node "$PARITY_NORMALIZER" "$malformed_output"

  [ "$status" -ne 0 ]
  [[ "$output" == *"$malformed_output"* ]]
  [[ "$output" == *"record=2"* ]]
  [[ "$output" == *"invalid JSON"* ]]
}

@test "development-discipline parity reports normalization failures before cleanup" {
  local fake_bin="$BATS_TEST_TMPDIR/parity-fake-bin"
  local real_node
  local real_rm
  local toolchain
  local side
  local opposite_side
  local marker_line
  local cleanup_line
  local line_index

  real_node="$(command -v node)"
  real_rm="$(command -v rm)"
  toolchain="$(awk -F'"' '/^channel = "/ { print $2; exit }' "$ROOT/plugins/development-discipline/rust/rust-toolchain.toml")"
  mkdir -p "$fake_bin"
  printf '%s\n' \
    '#!/bin/sh' \
    'if [ "$1" = "toolchain" ] && [ "$2" = "list" ]; then' \
    '  printf "%s\n" "$PARITY_TOOLCHAIN-x86_64-unknown-linux-gnu (active)"' \
    '  exit 0' \
    'fi' \
    'if [ "$1" = "run" ]; then' \
    '  exit 0' \
    'fi' \
    'exit 1' >"$fake_bin/rustup"
  printf '%s\n' \
    '#!/bin/sh' \
    'if [ "$1" = "$PARITY_FLOW_SCRIPT" ]; then' \
    '  case "$2" in' \
    '    */dist/*) side=distribution ;;' \
    '    *) side=source ;;' \
    '  esac' \
    '  printf "%s\n" '\''{"jsonrpc":"2.0","id":1,"result":{}}'\''' \
    '  if [ "$side" = "$PARITY_FAIL_SIDE" ]; then' \
    '    printf "%s\n" '\''{"jsonrpc":"2.0","id":2,"result":'\''' \
    '  fi' \
    '  exit 0' \
    'fi' \
    'exec "$REAL_NODE" "$@"' >"$fake_bin/node"
  printf '%s\n' \
    '#!/bin/sh' \
    'echo "parity-test-cleanup-started=true"' \
    'exec "$REAL_RM" "$@"' >"$fake_bin/rm"
  chmod +x "$fake_bin/rustup" "$fake_bin/node" "$fake_bin/rm"

  for side in source distribution; do
    if [ "$side" = "source" ]; then
      opposite_side="distribution"
    else
      opposite_side="source"
    fi

    run env \
      PATH="$fake_bin:$PATH" \
      REAL_NODE="$real_node" \
      REAL_RM="$real_rm" \
      PARITY_FLOW_SCRIPT="$ROOT/scripts/tests/development-discipline-mcp-flow.mjs" \
      PARITY_FAIL_SIDE="$side" \
      PARITY_TOOLCHAIN="$toolchain" \
      CARGO_HOME="$BATS_TEST_TMPDIR/cargo-$side" \
      RUSTUP_HOME="$BATS_TEST_TMPDIR/rustup-$side" \
      bash "$PARITY_CHECK"

    [ "$status" -ne 0 ]
    [[ "$output" == *"development-discipline-release-parity-normalization-failed=true side=$side"* ]]
    [[ "$output" != *"development-discipline-release-parity-normalization-failed=true side=$opposite_side"* ]]

    marker_line=-1
    cleanup_line=-1
    for line_index in "${!lines[@]}"; do
      if [ "${lines[$line_index]}" = "development-discipline-release-parity-normalization-failed=true side=$side" ]; then
        marker_line="$line_index"
      elif [ "${lines[$line_index]}" = "parity-test-cleanup-started=true" ]; then
        cleanup_line="$line_index"
      fi
    done
    [ "$marker_line" -ge 0 ]
    [ "$cleanup_line" -gt "$marker_line" ]
  done
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
