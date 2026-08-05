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
  cp "$BATS_TEST_DIRNAME/../../lefthook.yml" "$REPO/lefthook.yml"
  chmod +x "$REPO"/scripts/*.sh
  git -C "$REPO" add lefthook.yml scripts
  git -C "$REPO" commit -q -m seed
}

teardown() {
  chmod -R u+w "$REPO" 2>/dev/null || true
  rm -rf "$REPO"
}

install_repo() {
  (cd "$REPO" && scripts/install-worktree-hooks.sh)
}

write_managed_guard_launcher() {
  local hook_name="$1"
  local target="$REPO/.git/hooks/$hook_name"
  mkdir -p "$(dirname "$target")"
  {
    printf '%s\n' '#!/usr/bin/env bash'
    printf '# ai-plugins-managed-hook:v1:%s\n' "$hook_name"
    printf '%s\n' 'set -euo pipefail'
    printf '%s\n' "LEFTHOOK_ROOT_NAME='old-root'"
    printf '%s\n' "SAFETY_SCRIPT='scripts/worktree-guard.sh'"
    printf '%s\n' "SAFETY_TOKEN='worktree-guard'"
    printf '%s\n' 'REPO_ROOT=$(git rev-parse --show-toplevel)'
    printf '%s\n' 'COMMON_DIR=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)'
    printf '%s\n' 'LEFTHOOK_BIN="$COMMON_DIR/lefthook/roots/$LEFTHOOK_ROOT_NAME/bin/lefthook"'
    printf '%s\n' 'LEFTHOOK_CONFIG="$COMMON_DIR/lefthook/lefthook.yml"'
    printf '%s\n' 'unset AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN'
    printf '%s\n' '"$REPO_ROOT/$SAFETY_SCRIPT" "$@"'
    printf '%s\n' 'AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN="$SAFETY_TOKEN"'
    printf '%s\n' 'export AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN LEFTHOOK_CONFIG'
    printf 'exec "$LEFTHOOK_BIN" run "%s" --no-auto-install "$@"\n' "$hook_name"
  } >"$target"
  chmod +x "$target"
}

@test "installs local checks and optional worktree bootstrap" {
  run install_repo

  [ "$status" -eq 0 ]
  [ -x "$REPO/.git/hooks/pre-commit" ]
  [ -x "$REPO/.git/hooks/post-checkout" ]
  grep -Fq 'run "pre-commit"' "$REPO/.git/hooks/pre-commit"
  grep -Fq 'worktree-bootstrap.sh' "$REPO/.git/hooks/post-checkout"
}

@test "primary checkout commits succeed after installation" {
  install_repo

  run git -C "$REPO" commit -q --allow-empty -m permitted

  [ "$status" -eq 0 ]
}

@test "linked worktrees receive repository bootstrap state" {
  install_repo

  run git -C "$REPO" worktree add -q "$REPO/linked" -b linked

  [ "$status" -eq 0 ]
  [ -f "$REPO/linked/.env.worktree" ]
  [ -f "$REPO/linked/.envrc" ]
}

@test "refresh replaces managed guard launchers" {
  write_managed_guard_launcher pre-commit
  write_managed_guard_launcher pre-push

  run install_repo

  [ "$status" -eq 0 ]
  [ -x "$REPO/.git/hooks/pre-commit" ]
  run git -C "$REPO" commit -q --allow-empty -m refreshed
  [ "$status" -eq 0 ]
}

@test "foreign hooks are archived before replacement" {
  mkdir -p "$REPO/.git/hooks"
  printf '#!/usr/bin/env bash\necho foreign\n' >"$REPO/.git/hooks/pre-commit"
  chmod +x "$REPO/.git/hooks/pre-commit"

  run install_repo

  [ "$status" -eq 0 ]
  grep -qx 'echo foreign' "$REPO/.git/hooks/pre-commit.worktrees-backup"
  grep -Fq 'run "pre-commit"' "$REPO/.git/hooks/pre-commit"
}
