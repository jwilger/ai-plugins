#!/usr/bin/env bats

setup() {
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
}

teardown() {
  rm -rf -- "$REPO"
}

@test "the retired guard compatibility shim permits the primary checkout" {
  local guard="$BATS_TEST_DIRNAME/../worktree-guard.sh"

  run bash -c "cd '$REPO' && '$guard'"

  [ "$status" -eq 0 ]
}
