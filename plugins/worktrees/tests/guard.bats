#!/usr/bin/env bats
# Tests for the sidequest enforcement guard.

setup() {
  GUARD="$BATS_TEST_DIRNAME/../scripts/sidequest-guard.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  git -C "$REPO" commit -q --allow-empty -m seed
  cp "$GUARD" "$REPO/.git/hooks/pre-commit"
  chmod +x "$REPO/.git/hooks/pre-commit"
}

teardown() {
  rm -rf "$REPO"
}

@test "blocks a commit from the main checkout" {
  run git -C "$REPO" commit -q --allow-empty -m change
  [ "$status" -ne 0 ]
}

@test "allows a commit from a linked worktree" {
  git -C "$REPO" worktree add -q "$REPO/.worktrees/feat" -b feat
  run git -C "$REPO/.worktrees/feat" commit -q --allow-empty -m change
  [ "$status" -eq 0 ]
}
