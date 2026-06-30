#!/usr/bin/env bats
# Tests for the per-worktree port allocator.

setup() {
  ALLOC="$BATS_TEST_DIRNAME/../scripts/worktree-ports.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  git -C "$REPO" commit -q --allow-empty -m seed
  git -C "$REPO" worktree add -q "$REPO/.worktrees/a" -b a
  git -C "$REPO" worktree add -q "$REPO/.worktrees/b" -b b
}

teardown() {
  rm -rf "$REPO"
}

@test "allocates the first slot and is idempotent" {
  first=$(bash "$ALLOC" "$REPO/.worktrees/a")
  again=$(bash "$ALLOC" "$REPO/.worktrees/a")
  [ "$first" = "$again" ]
  echo "$first" | grep -qx "PORT=4100"
  echo "$first" | grep -qx "PG_PORT=5500"
}

@test "gives distinct worktrees distinct port blocks" {
  a=$(bash "$ALLOC" "$REPO/.worktrees/a" | grep '^PORT=')
  b=$(bash "$ALLOC" "$REPO/.worktrees/b" | grep '^PORT=')
  [ "$a" != "$b" ]
}

@test "honors a configured base port" {
  out=$(SIDEQUEST_PORT_BASE_HTTP=8000 bash "$ALLOC" "$REPO/.worktrees/a")
  echo "$out" | grep -qx "PORT=8000"
}
