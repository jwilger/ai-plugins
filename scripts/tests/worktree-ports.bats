#!/usr/bin/env bats

setup() {
  ALLOC="$BATS_TEST_DIRNAME/../worktree-ports.sh"
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

@test "allocates an idempotent first slot" {
  first=$(bash "$ALLOC" "$REPO/.worktrees/a")
  again=$(bash "$ALLOC" "$REPO/.worktrees/a")

  [ "$first" = "$again" ]
  echo "$first" | grep -qx "PORT=4100"
  echo "$first" | grep -qx "PG_PORT=5500"
}

@test "assigns distinct slots to distinct worktrees" {
  a=$(bash "$ALLOC" "$REPO/.worktrees/a" | grep '^PORT=')
  b=$(bash "$ALLOC" "$REPO/.worktrees/b" | grep '^PORT=')

  [ "$a" != "$b" ]
}

@test "honors configured base ports and stride" {
  out=$(WORKTREE_PORT_BASE_HTTP=8000 WORKTREE_PORT_BASE_PG=9000 WORKTREE_PORT_STRIDE=100 bash "$ALLOC" "$REPO/.worktrees/b")

  echo "$out" | grep -qx "PORT=8000"
  echo "$out" | grep -qx "PG_PORT=9000"
}
