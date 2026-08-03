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
  event_file="$(find "$project/.development-system/tiber/store/events" -name '*.jsonl' -print -quit)"
  [ -n "$event_file" ]
  run rg --fixed-strings 'Restore deterministic workflow' "$event_file"
  [ "$status" -eq 0 ]
  [ -z "$(find "$project" -name '*.md' -print -quit)" ]
}

@test "Tiber lists ticket titles by replaying Eventcore transactions" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Replay this ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"Replay this ticket"* ]]
}

@test "Tiber list exposes stable ticket identifiers from Eventcore streams" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Addressable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"id=ticket-"* ]]
  [[ "$output" == *"title=Addressable ticket"* ]]
}

@test "Tiber claims an addressable ticket through Eventcore" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Claimable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"
  [ -n "$ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=alice"* ]]
}

@test "Tiber preserves the first owner when a second claim is rejected" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Exclusively claimable"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=alice"* ]]
  [[ "$output" != *"owner=bob"* ]]
}

@test "Tiber releases only the current owner's claim and permits a new claim" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Releasable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"
  [ -n "$ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber release "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber release "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=bob"* ]]
}
