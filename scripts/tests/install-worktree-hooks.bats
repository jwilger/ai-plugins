#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  INSTALL="$BATS_TEST_DIRNAME/../install-worktree-hooks.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name test
  git -C "$REPO" config commit.gpgsign false
  mkdir -p "$REPO/scripts"
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
  rm -rf -- "$REPO"
}

install_repo() {
  (cd "$REPO" && scripts/install-worktree-hooks.sh)
}

hook_target() {
  local hook_name="$1"
  printf '%s/.git/hooks/%s\n' "$REPO" "$hook_name"
}

create_foreign_hook() {
  local hook_name="$1"
  local target_file
  target_file="$(hook_target "$hook_name")"
  mkdir -p "$(dirname "$target_file")"
  printf '#!/usr/bin/env bash\necho foreign-%s\n' "$hook_name" >"$target_file"
  chmod +x "$target_file"
}

write_legacy_managed_hook() {
  local hook_name="$1"
  local target_file
  target_file="$(hook_target "$hook_name")"
  mkdir -p "$(dirname "$target_file")"

  {
    printf '%s\n' '#!/usr/bin/env bash'
    printf '# ai-plugins-managed-hook:v1:%s\n' "$hook_name"
    printf '%s\n' 'set -euo pipefail'
    printf '%s\n' "LEFTHOOK_ROOT_NAME='legacy-root'"
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
  } >"$target_file"
  chmod +x "$target_file"
}

lefthook_root() {
  printf '%s/.git/lefthook/roots/lefthook-%s\n' \
    "$REPO" "${AI_PLUGINS_LEFTHOOK_STORE_PATH##*/}"
}

@test "the repository installer is executable" {
  [ -x "$INSTALL" ]
}

@test "fails clearly when the flake-selected Lefthook is unavailable" {
  run -127 env \
    -u AI_PLUGINS_LEFTHOOK_BIN \
    -u AI_PLUGINS_LEFTHOOK_STORE_PATH \
    -u AI_PLUGINS_LEFTHOOK_VERSION \
    bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 127 ]
  [[ "$output" == *"lefthook_pinned_runtime_missing"* ]]
  [ ! -e "$(hook_target post-checkout)" ]
}

@test "only the main checkout may install the shared post-checkout hook" {
  local linked="$REPO/linked"
  git -C "$REPO" worktree add -q "$linked" -b linked-install

  run bash -c "cd '$linked' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 64 ]
  [[ "$output" == *"main_checkout_required"* ]]
  [ ! -e "$(hook_target post-checkout)" ]
}

@test "installs only a post-checkout launcher and a pinned snapshot" {
  local root

  run install_repo

  [ "$status" -eq 0 ]
  root="$(lefthook_root)"
  [ -L "$root" ]
  [ "$(readlink "$root")" = "$AI_PLUGINS_LEFTHOOK_STORE_PATH" ]
  [ -f "$REPO/.git/lefthook/lefthook.yml" ]
  cmp "$REPO/lefthook.yml" "$REPO/.git/lefthook/lefthook.yml"
  [ -x "$(hook_target post-checkout)" ]
  grep -Fq '# ai-plugins-managed-hook:v1:post-checkout' "$(hook_target post-checkout)"
  grep -Fq "scripts/worktree-bootstrap.sh" "$(hook_target post-checkout)"
  grep -Fq 'run "post-checkout"' "$(hook_target post-checkout)"
  ! grep -Fq 'pre-commit:' "$REPO/.git/lefthook/lefthook.yml"
  ! grep -Fq 'pre-push:' "$REPO/.git/lefthook/lefthook.yml"
  [ ! -e "$(hook_target pre-commit)" ]
  [ ! -e "$(hook_target pre-push)" ]
}

@test "ordinary main-checkout commits are permitted after installation" {
  install_repo

  run git -C "$REPO" commit -q --allow-empty -m permitted

  [ "$status" -eq 0 ]
}

@test "the post-checkout launcher still bootstraps linked worktrees" {
  local linked="$REPO/linked"
  install_repo

  run git -C "$REPO" worktree add -q "$linked" -b linked-use

  [ "$status" -eq 0 ]
  [ -f "$linked/.env.worktree" ]
  [ -f "$linked/.envrc" ]
}

@test "leaves foreign pre-commit and pre-push hooks untouched" {
  local pre_commit="$(hook_target pre-commit)"
  local pre_push="$(hook_target pre-push)"
  create_foreign_hook pre-commit
  create_foreign_hook pre-push
  cp "$pre_commit" "$REPO/original-pre-commit"
  cp "$pre_push" "$REPO/original-pre-push"

  run install_repo

  [ "$status" -eq 0 ]
  cmp "$REPO/original-pre-commit" "$pre_commit"
  cmp "$REPO/original-pre-push" "$pre_push"
  [ ! -e "$pre_commit.worktrees-backup" ]
  [ ! -e "$pre_push.worktrees-backup" ]
}

@test "removes only exact legacy managed pre-commit and pre-push launchers" {
  write_legacy_managed_hook pre-commit
  write_legacy_managed_hook pre-push

  run install_repo

  [ "$status" -eq 0 ]
  [ ! -e "$(hook_target pre-commit)" ]
  [ ! -e "$(hook_target pre-push)" ]
  [[ "$output" == *"legacy_managed_hook_removed: $(hook_target pre-commit)"* ]]
  [[ "$output" == *"legacy_managed_hook_removed: $(hook_target pre-push)"* ]]
}

@test "preserves a legacy-looking hook that is not an exact managed launcher" {
  local pre_commit="$(hook_target pre-commit)"
  write_legacy_managed_hook pre-commit
  printf '%s\n' '# user customization' >>"$pre_commit"

  run install_repo

  [ "$status" -eq 0 ]
  [ -f "$pre_commit" ]
  grep -Fq '# user customization' "$pre_commit"
}

@test "does not retire managed legacy hooks if post-checkout installation fails" {
  local post_checkout="$(hook_target post-checkout)"
  write_legacy_managed_hook pre-commit
  mkdir -p "$(dirname "$post_checkout")"
  mkfifo "$post_checkout"

  run install_repo

  [ "$status" -eq 65 ]
  [[ "$output" == *"hook_target_unsupported"* ]]
  [ -f "$(hook_target pre-commit)" ]
}

@test "backs up a foreign post-checkout hook before replacement" {
  local post_checkout="$(hook_target post-checkout)"
  create_foreign_hook post-checkout

  run install_repo

  [ "$status" -eq 0 ]
  grep -qx 'echo foreign-post-checkout' "$post_checkout.worktrees-backup"
  grep -Fq '# ai-plugins-managed-hook:v1:post-checkout' "$post_checkout"
  [[ "$output" == *"hook_backup_created"* ]]
}

@test "reinstalling the post-checkout launcher is idempotent" {
  local post_checkout="$(hook_target post-checkout)"
  local before
  install_repo
  before="$(sha256sum "$post_checkout")"

  run install_repo

  [ "$status" -eq 0 ]
  [ "$(sha256sum "$post_checkout")" = "$before" ]
  [ ! -e "$post_checkout.worktrees-backup" ]
}

@test "does not inspect unsupported pre-push targets" {
  local pre_push="$(hook_target pre-push)"
  mkdir -p "$(dirname "$pre_push")"
  mkfifo "$pre_push"

  run install_repo

  [ "$status" -eq 0 ]
  [ -p "$pre_push" ]
}

@test "local Lefthook overrides cannot rewrite the managed post-checkout launcher" {
  local post_checkout="$(hook_target post-checkout)"
  local before
  local head
  install_repo
  before="$(sha256sum "$post_checkout")"
  printf 'no_auto_install: false\n' >"$REPO/lefthook-local.yml"
  head="$(git -C "$REPO" rev-parse HEAD)"

  run bash -c "cd '$REPO' && .git/hooks/post-checkout '$head' '$head' 1"

  [ "$status" -eq 0 ]
  [ "$(sha256sum "$post_checkout")" = "$before" ]
}
