#!/usr/bin/env bats
# Tests for the session-end notice.

setup() {
  SCRIPT="$BATS_TEST_DIRNAME/../hooks/session-end-notice.sh"
  WORK="$(mktemp -d)"
  mkdir -p "$WORK/.git/sidequest"
}

teardown() {
  rm -rf "$WORK"
}

@test "notices a running side-quest" {
  cat >"$WORK/.git/sidequest/registry.json" <<'JSON'
[{ "state": "running", "branch": "side-quest/x" }]
JSON
  run bash -c "cd '$WORK' && '$SCRIPT' 2>&1"
  [ "$status" -eq 0 ]
  [[ "$output" == *"still running"* ]]
}

@test "is silent when nothing is running" {
  cat >"$WORK/.git/sidequest/registry.json" <<'JSON'
[{ "state": "delivered", "branch": "side-quest/x" }]
JSON
  run bash -c "cd '$WORK' && '$SCRIPT' 2>&1"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "is silent with no registry" {
  run bash -c "cd '$WORK' && '$SCRIPT' 2>&1"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}
