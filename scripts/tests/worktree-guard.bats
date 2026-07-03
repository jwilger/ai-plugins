#!/usr/bin/env bats

setup() {
  GUARD="$BATS_TEST_DIRNAME/../worktree-guard.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  git -C "$REPO" commit -q --allow-empty -m seed
}

teardown() {
  rm -rf "$REPO"
}

@test "blocks the main checkout" {
  run bash -c "cd '$REPO' && '$GUARD'"

  [ "$status" -ne 0 ]
  [[ "$output" == *"main checkout"* ]]
}

@test "allows a linked worktree" {
  git -C "$REPO" worktree add -q "$REPO/.worktrees/feat" -b feat

  run bash -c "cd '$REPO/.worktrees/feat' && '$GUARD'"

  [ "$status" -eq 0 ]
}
