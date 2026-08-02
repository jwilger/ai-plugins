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

@test "Tiber initializes an Eventcore transaction store instead of ticket Markdown files" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [ -d "$project/.development-system/tiber/store/events" ]
  [ -z "$(find "$project/.development-system/tiber/store/events" -name '*.md' -print -quit)" ]
}

@test "Tiber creates a functional ticket as an immutable Eventcore transaction" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Restore deterministic workflow"' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"Restore deterministic workflow"* ]]
  [ -n "$(find "$project/.development-system/tiber/store/events" -name '*.jsonl' -print -quit)" ]
  [ -z "$(find "$project" -name '*.md' -print -quit)" ]
}
