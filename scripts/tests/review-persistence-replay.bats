#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  SCRIPT="$ROOT/plugins/development-system/scripts/replay-review-operation.sh"
  VERSION="$(jq -r .version "$ROOT/plugins/development-system/plugin.json")"
  INSTALL="$BATS_TEST_TMPDIR/data/ai-plugins/development-system/$VERSION/linux-x86_64"
  mkdir -p "$INSTALL"
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$@"\ncat\n' >"$INSTALL/development-discipline-mcp"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$INSTALL/tiber"
  chmod +x "$INSTALL/"{development-discipline-mcp,tiber}
  printf '%s\n' "$VERSION" >"$INSTALL/.plugin-version"
}

@test "replay launcher rejects mismatched binaries without installing or executing" {
  printf '%s\n' stale >"$INSTALL/.plugin-version"
  run env PLUGIN_DATA="$BATS_TEST_TMPDIR/data" DEVELOPMENT_SYSTEM_HOST_OVERRIDE=linux-x86_64 \
    bash "$SCRIPT" "$BATS_TEST_TMPDIR" final_review.plan
  [ "$status" -ne 0 ]
  [[ "$output" == *replay_binary_version_mismatch* ]]
  [ "$(cat "$INSTALL/.plugin-version")" = stale ]
}

@test "replay launcher preserves argument and JSON bytes for the version matched executable" {
  run bash -c 'printf "%s" '\''{"session_id":"spaces and $literal"}'\'' | env PLUGIN_DATA="$1/data" DEVELOPMENT_SYSTEM_HOST_OVERRIDE=linux-x86_64 bash "$2" "$1/path with spaces" final_review.advance' _ "$BATS_TEST_TMPDIR" "$SCRIPT"
  [ "$status" -eq 0 ]
  [[ "$output" == *--replay-review-operation* ]]
  [[ "$output" == *"$BATS_TEST_TMPDIR/path with spaces"* ]]
  [[ "$output" == *'"spaces and $literal"'* ]]
}
