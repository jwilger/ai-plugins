#!/usr/bin/env bats

setup() {
  PLUGIN_ROOT="$(cd "$BATS_TEST_DIRNAME/../../plugins/development-system" && pwd)"
  CLI="$PLUGIN_ROOT/bin/development-system"
}

@test "installed development-system neither ships nor advertises retired Tiber migration compatibility" {
  run "$CLI" --help

  [ "$status" -eq 0 ]
  [[ "${output,,}" != *"tiber"* ]]

  run "$CLI" migrate-tiber-to-beads

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.usage command=migrate-tiber-to-beads"* ]]

  run find "$PLUGIN_ROOT" -iname '*tiber*' -print

  [ "$status" -eq 0 ]
  [ -z "$output" ]

  run rg --hidden --ignore-case --files-with-matches 'tiber' "$PLUGIN_ROOT"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
}
