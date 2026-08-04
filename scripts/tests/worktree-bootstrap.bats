#!/usr/bin/env bats

setup() {
  BOOTSTRAP="$BATS_TEST_DIRNAME/../worktree-bootstrap.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  mkdir -p "$REPO/scripts" "$REPO/.dependencies/npm/bin"
  cp "$BATS_TEST_DIRNAME/../worktree-ports.sh" "$REPO/scripts/worktree-ports.sh"
  cp "$BOOTSTRAP" "$REPO/scripts/worktree-bootstrap.sh"
  chmod +x "$REPO/scripts/worktree-ports.sh" "$REPO/scripts/worktree-bootstrap.sh"
  touch "$REPO/.dependencies/npm/bin/example-tool"
  git -C "$REPO" add scripts
  git -C "$REPO" commit -q -m seed
}

teardown() {
  rm -rf "$REPO"
}

@test "is inert in the main checkout" {
  run bash -c "cd '$REPO' && scripts/worktree-bootstrap.sh"

  [ "$status" -eq 0 ]
  [ ! -f "$REPO/.env.worktree" ]
}

@test "writes per-worktree env and direnv config in linked worktrees" {
  git -C "$REPO" worktree add -q "$REPO/.worktrees/feat" -b feat

  run bash -c "cd '$REPO/.worktrees/feat' && scripts/worktree-bootstrap.sh"

  [ "$status" -eq 0 ]
  grep -qx "PORT=4100" "$REPO/.worktrees/feat/.env.worktree"
  grep -qx "PG_PORT=5500" "$REPO/.worktrees/feat/.env.worktree"
  grep -qx "AI_PLUGINS_MAIN_CHECKOUT=$REPO" "$REPO/.worktrees/feat/.env.worktree"
  grep -qx "use flake" "$REPO/.worktrees/feat/.envrc"
}

@test "warms project-local dependency cache" {
  git -C "$REPO" worktree add -q "$REPO/.worktrees/cache" -b cache

  run bash -c "cd '$REPO/.worktrees/cache' && scripts/worktree-bootstrap.sh"

  [ "$status" -eq 0 ]
  [ -f "$REPO/.worktrees/cache/.dependencies/npm/bin/example-tool" ]
}

@test "does not copy disposable eval state into linked worktrees" {
  mkdir -p "$REPO/.dependencies/evals/codex-home-full-marketplace"
  touch "$REPO/.dependencies/evals/codex-home-full-marketplace/auth.json"
  git -C "$REPO" worktree add -q "$REPO/.worktrees/evals" -b evals

  run bash -c "cd '$REPO/.worktrees/evals' && scripts/worktree-bootstrap.sh"

  [ "$status" -eq 0 ]
  [ ! -e "$REPO/.worktrees/evals/.dependencies/evals" ]
}

@test "does not mark bootstrap complete when cache warmup fails" {
  git -C "$REPO" worktree add -q "$REPO/.worktrees/cp-fails" -b cp-fails
  mkdir -p "$REPO/.worktrees/cp-fails/fake-bin"
  cat >"$REPO/.worktrees/cp-fails/fake-bin/cp" <<'SH'
#!/usr/bin/env bash
exit 42
SH
  chmod +x "$REPO/.worktrees/cp-fails/fake-bin/cp"
  marker="$(git -C "$REPO/.worktrees/cp-fails" rev-parse --git-dir)/.ai-plugins-worktree-bootstrapped"

  run bash -c "cd '$REPO/.worktrees/cp-fails' && PATH='$REPO/.worktrees/cp-fails/fake-bin':\$PATH scripts/worktree-bootstrap.sh"

  [ "$status" -eq 42 ]
  [ ! -f "$marker" ]
}
