#!/usr/bin/env bats

setup() {
  TEARDOWN="$BATS_TEST_DIRNAME/../worktree-teardown.sh"
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

@test "validates the worktree path before loading env or stopping services" {
  bogus="$REPO/not-a-worktree"
  mkdir -p "$bogus" "$REPO/fake-bin"
  printf 'COMPOSE_PROJECT_NAME=must-not-run\n' >"$bogus/.env.worktree"
  cat >"$REPO/fake-bin/docker" <<'SH'
#!/usr/bin/env bash
echo docker-called >>"$DOCKER_LOG"
SH
  chmod +x "$REPO/fake-bin/docker"

  run bash -c "DOCKER_LOG='$REPO/docker.log' PATH='$REPO/fake-bin':\$PATH '$TEARDOWN' '$bogus'"

  [ "$status" -ne 0 ]
  [ ! -f "$REPO/docker.log" ]
}

@test "rejects subdirectories before loading env or stopping services" {
  mkdir -p "$REPO/.worktrees/a/subdir" "$REPO/fake-bin"
  printf 'COMPOSE_PROJECT_NAME=must-not-run\n' >"$REPO/.worktrees/a/subdir/.env.worktree"
  cat >"$REPO/fake-bin/docker" <<'SH'
#!/usr/bin/env bash
echo docker-called >>"$DOCKER_LOG"
SH
  chmod +x "$REPO/fake-bin/docker"

  run bash -c "DOCKER_LOG='$REPO/docker.log' PATH='$REPO/fake-bin':\$PATH '$TEARDOWN' '$REPO/.worktrees/a/subdir'"

  [ "$status" -ne 0 ]
  [ ! -f "$REPO/docker.log" ]
}

@test "releases the worktree port slot during teardown" {
  first=$(bash "$ALLOC" "$REPO/.worktrees/a")
  bash "$ALLOC" "$REPO/.worktrees/b" >/dev/null

  run bash "$TEARDOWN" "$REPO/.worktrees/a"

  [ "$status" -eq 0 ]
  git -C "$REPO" worktree remove "$REPO/.worktrees/a"
  git -C "$REPO" worktree add -q "$REPO/.worktrees/c" -b c
  reused=$(bash "$ALLOC" "$REPO/.worktrees/c")
  [ "$reused" = "$first" ]
}
