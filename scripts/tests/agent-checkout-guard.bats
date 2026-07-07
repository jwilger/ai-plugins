#!/usr/bin/env bats

setup() {
  GUARD="$BATS_TEST_DIRNAME/../agent-checkout-guard.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q -b main
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  printf 'seed\n' >"$REPO/file.txt"
  git -C "$REPO" add file.txt
  git -C "$REPO" commit -q -m seed
}

teardown() {
  rm -rf "$REPO" "${REMOTE:-}" "${OTHER:-}"
}

@test "blocks a clean main checkout" {
  run bash -c "cd '$REPO' && '$GUARD'"

  [ "$status" -eq 1 ]
  [[ "$output" == *"main checkout is coordination-only"* ]]
  [[ "$output" == *"git worktree add .worktrees/<branch-name> -b <branch-name>"* ]]
}

@test "allows a linked worktree" {
  git -C "$REPO" worktree add -q "$REPO/.worktrees/feat" -b feat

  run bash -c "cd '$REPO/.worktrees/feat' && '$GUARD'"

  [ "$status" -eq 0 ]
}

@test "blocks dirty local changes in the main checkout" {
  printf 'local\n' >>"$REPO/file.txt"

  run bash -c "cd '$REPO' && '$GUARD'"

  [ "$status" -eq 1 ]
  [[ "$output" == *"main checkout already has local changes"* ]]
}

@test "identifies dirty changes that match upstream" {
  REMOTE="$(mktemp -d)"
  OTHER="$(mktemp -d)"
  git init -q --bare "$REMOTE"
  git --git-dir="$REMOTE" symbolic-ref HEAD refs/heads/main
  git -C "$REPO" remote add origin "$REMOTE"
  git -C "$REPO" push -q -u origin main

  git clone -q "$REMOTE" "$OTHER"
  git -C "$OTHER" config user.email test@example.com
  git -C "$OTHER" config user.name test
  git -C "$OTHER" config commit.gpgsign false
  printf 'upstream\n' >"$OTHER/file.txt"
  printf 'new from upstream\n' >"$OTHER/new-file.txt"
  git -C "$OTHER" add file.txt new-file.txt
  git -C "$OTHER" commit -q -m upstream-change
  git -C "$OTHER" push -q

  git -C "$REPO" fetch -q origin main
  printf 'upstream\n' >"$REPO/file.txt"
  printf 'new from upstream\n' >"$REPO/new-file.txt"

  run bash -c "cd '$REPO' && '$GUARD'"

  [ "$status" -eq 1 ]
  [[ "$output" == *"dirty worktree matches upstream 'origin/main'"* ]]
}
