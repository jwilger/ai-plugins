#!/usr/bin/env bats

setup() {
  INSTALL="$BATS_TEST_DIRNAME/../install-worktree-hooks.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  mkdir -p "$REPO/scripts"
  cp "$BATS_TEST_DIRNAME/../worktree-guard.sh" "$REPO/scripts/worktree-guard.sh"
  cp "$BATS_TEST_DIRNAME/../worktree-bootstrap.sh" "$REPO/scripts/worktree-bootstrap.sh"
  cp "$BATS_TEST_DIRNAME/../worktree-ports.sh" "$REPO/scripts/worktree-ports.sh"
  cp "$INSTALL" "$REPO/scripts/install-worktree-hooks.sh"
  chmod +x "$REPO"/scripts/*.sh
  git -C "$REPO" add scripts
  git -C "$REPO" commit -q -m seed
}

teardown() {
  rm -rf "$REPO"
}

@test "installs executable shared hooks" {
  run bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 0 ]
  [ -x "$REPO/.git/hooks/pre-commit" ]
  [ -x "$REPO/.git/hooks/pre-push" ]
  [ -x "$REPO/.git/hooks/post-checkout" ]
}

@test "installed guard blocks main-checkout commits" {
  bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  run git -C "$REPO" commit -q --allow-empty -m blocked

  [ "$status" -ne 0 ]
}

@test "backs up pre-existing hooks before replacing them" {
  mkdir -p "$REPO/.git/hooks"
  printf '#!/usr/bin/env bash\necho existing\n' >"$REPO/.git/hooks/pre-commit"
  chmod +x "$REPO/.git/hooks/pre-commit"

  run bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 0 ]
  grep -qx "echo existing" "$REPO/.git/hooks/pre-commit.worktrees-backup"
  grep -q "worktree-guard.sh" "$REPO/.git/hooks/pre-commit"
  [[ "$output" == *"backed up existing hook"* ]]
}
