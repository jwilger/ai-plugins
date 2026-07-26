#!/usr/bin/env bats

setup() {
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  export REPO_ROOT
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

@test "setup refuses a linked worktree and identifies the primary checkout" {
  git -C "$TEST_ROOT" init --initial-branch=main primary
  git -C "$TEST_ROOT/primary" config user.email test@example.com
  git -C "$TEST_ROOT/primary" config user.name "Test User"
  touch "$TEST_ROOT/primary/README.md"
  git -C "$TEST_ROOT/primary" add README.md
  git -C "$TEST_ROOT/primary" commit -m "test: initialize fixture"
  git -C "$TEST_ROOT/primary" worktree add -b feature "$TEST_ROOT/linked"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/linked" \
    --preset personal-trunk \
    --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.setup_requires_primary_checkout"* ]]
  [[ "$output" == *"$TEST_ROOT/primary"* ]]
  [ ! -e "$TEST_ROOT/linked/.development-system.toml" ]
}

@test "setup refuses a bare repository because it is not a checkout" {
  git -C "$TEST_ROOT" init --bare bare.git

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/bare.git" \
    --preset personal-trunk \
    --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.setup_requires_checkout"* ]]
  [ ! -e "$TEST_ROOT/bare.git/.development-system.toml" ]
}

@test "setup reports a stable error when an option value is missing" {
  run "$REPO_ROOT/plugins/development-system/bin/development-system" setup --project

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.missing_option_value option=--project"* ]]
}

@test "setup does not consume another option as a missing value" {
  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project \
    --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.missing_option_value option=--project"* ]]
}
