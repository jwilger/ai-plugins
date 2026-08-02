#!/usr/bin/env bats

setup() {
  PLUGIN_ROOT="$(cd "$BATS_TEST_DIRNAME/../../plugins/development-system" && pwd)"
  CLI="$PLUGIN_ROOT/bin/development-system"
}

@test "installed development-system exposes its bundled Tiber tracker" {
  run "$CLI" tiber --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Repository-local task board"* ]]

  run find "$PLUGIN_ROOT" -path '*/components/tiber/bin/tiber' -type f -print

  [ "$status" -eq 0 ]
  [ -n "$output" ]
}
