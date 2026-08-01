#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TEST_ROOT="$(mktemp -d)"
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

@test "active repository surfaces contain no Pi runtime or package support" {
  run bash "$REPO_ROOT/scripts/check-no-pi-support.sh"

  [ "$status" -eq 0 ]
  [ ! -e "$REPO_ROOT/package.json" ]
  [ ! -e "$REPO_ROOT/plugins/development-system/package.json" ]
  [ ! -e "$REPO_ROOT/plugins/development-system/extensions/development-system" ]
  [ ! -e "$REPO_ROOT/.agents/plugins/pi-support.json" ]
}

@test "setup defaults worktree-capable repositories to concurrent tickets" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  git -C "$TEST_ROOT/project" config commit.gpgsign false
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -m "test: initialize fixture"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *'mode = "concurrent-tickets"'* ]]
}

@test "doctor recognizes only the supported Codex and Claude harnesses" {
  mkdir -p "$TEST_ROOT/project"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project" \
    --harness pi

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.unsupported_harness harness=pi"* ]]
}
