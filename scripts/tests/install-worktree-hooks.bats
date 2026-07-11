#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  INSTALL="$BATS_TEST_DIRNAME/../install-worktree-hooks.sh"
  REPO="$(mktemp -d)"
  CRASH_GROUP=""
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
  if [ -n "$CRASH_GROUP" ]; then
    kill -KILL -- "-$CRASH_GROUP" 2>/dev/null || true
    wait "$CRASH_GROUP" 2>/dev/null || true
  fi
  chmod -R u+w "$REPO" 2>/dev/null || true
  rm -rf "$REPO"
}

install_repo() {
  (cd "$REPO" && scripts/install-worktree-hooks.sh)
}

create_foreign_hook() {
  local hook_name="${1:-pre-commit}"
  mkdir -p "$REPO/.git/hooks"
  printf '#!/usr/bin/env bash\necho foreign-%s\n' "$hook_name" >"$REPO/.git/hooks/$hook_name"
  chmod +x "$REPO/.git/hooks/$hook_name"
}

hook_fingerprints() {
  sha256sum \
    "$REPO/.git/hooks/pre-commit" \
    "$REPO/.git/hooks/pre-push" \
    "$REPO/.git/hooks/post-checkout"
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
  [ ! -e "$REPO/.git/lefthook" ]
}

@test "rejects a mismatched Lefthook version before mutation" {
  mkdir -p "$REPO/fake-bin"
  printf '#!/usr/bin/env bash\nprintf "9.9.9\\n"\n' >"$REPO/fake-bin/lefthook"
  chmod +x "$REPO/fake-bin/lefthook"

  run env \
    AI_PLUGINS_LEFTHOOK_BIN="$REPO/fake-bin/lefthook" \
    AI_PLUGINS_LEFTHOOK_VERSION="2.1.5" \
    bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 65 ]
  [[ "$output" == *"lefthook_version_mismatch"* ]]
  [ ! -e "$REPO/.git/lefthook" ]
  [ ! -e "$REPO/.git/hooks/pre-commit" ]
}

@test "fails without mutation when core.hooksPath is configured" {
  git -C "$REPO" config core.hooksPath .githooks
  mkdir -p "$REPO/.githooks"
  printf '#!/usr/bin/env bash\necho custom-path\n' >"$REPO/.githooks/pre-commit"
  chmod +x "$REPO/.githooks/pre-commit"

  run install_repo

  [ "$status" -eq 65 ]
  [[ "$output" == *"core_hooks_path_configured"* ]]
  grep -qx 'echo custom-path' "$REPO/.githooks/pre-commit"
  [ ! -e "$REPO/.git/lefthook" ]
}

@test "only the main checkout may install shared hooks" {
  local linked="$REPO/linked"
  git -C "$REPO" worktree add -q "$linked" -b linked-install

  run bash -c "cd '$linked' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 64 ]
  [[ "$output" == *"main_checkout_required"* ]]
  [ ! -e "$REPO/.git/lefthook" ]
  [ ! -e "$REPO/.git/hooks/pre-commit" ]
}

@test "invalid Lefthook configuration fails before hook mutation" {
  create_foreign_hook pre-commit
  cp "$REPO/.git/hooks/pre-commit" "$REPO/original-hook"
  printf 'pre-commit: [\n' >"$REPO/lefthook.yml"

  run install_repo

  [ "$status" -ne 0 ]
  [[ "$output" == *"lefthook_config_invalid"* ]]
  cmp "$REPO/original-hook" "$REPO/.git/hooks/pre-commit"
  [ ! -e "$REPO/.git/hooks/pre-commit.worktrees-backup" ]
}

@test "installs three mapped launchers, a config snapshot, and an exact Nix GC root" {
  local root

  run install_repo

  [ "$status" -eq 0 ]
  root="$(lefthook_root)"
  [ -L "$root" ]
  [ "$(readlink "$root")" = "$AI_PLUGINS_LEFTHOOK_STORE_PATH" ]
  nix-store --query --roots "$AI_PLUGINS_LEFTHOOK_STORE_PATH" | grep -Fq "$root"
  [ -f "$REPO/.git/lefthook/lefthook.yml" ]
  [ "$(stat -c '%a' "$REPO/.git/lefthook/lefthook.yml")" = 444 ]
  cmp "$REPO/lefthook.yml" "$REPO/.git/lefthook/lefthook.yml"

  for hook in pre-commit pre-push post-checkout; do
    local delegation_line
    local safety_line
    local suppression_line

    [ -x "$REPO/.git/hooks/$hook" ]
    grep -Fq "# ai-plugins-managed-hook:v1:$hook" "$REPO/.git/hooks/$hook"
    grep -Fq 'LEFTHOOK_CONFIG="$COMMON_DIR/lefthook/lefthook.yml"' "$REPO/.git/hooks/$hook"
    grep -Fq -- '--no-auto-install' "$REPO/.git/hooks/$hook"
    ! grep -Fq -- '--file' "$REPO/.git/hooks/$hook"

    safety_line="$(grep -nF '"$REPO_ROOT/$SAFETY_SCRIPT" "$@"' "$REPO/.git/hooks/$hook" | cut -d: -f1)"
    suppression_line="$(grep -nF 'export AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN LEFTHOOK_CONFIG' "$REPO/.git/hooks/$hook" | cut -d: -f1)"
    delegation_line="$(grep -nF 'exec "$LEFTHOOK_BIN" run' "$REPO/.git/hooks/$hook" | cut -d: -f1)"
    [ -n "$safety_line" ]
    [ "$safety_line" -lt "$suppression_line" ]
    [ "$suppression_line" -lt "$delegation_line" ]
  done

  grep -Fq 'scripts/worktree-guard.sh' "$REPO/.git/hooks/pre-commit"
  grep -Fq 'run "pre-commit"' "$REPO/.git/hooks/pre-commit"
  grep -Fq 'scripts/worktree-guard.sh' "$REPO/.git/hooks/pre-push"
  grep -Fq 'run "pre-push"' "$REPO/.git/hooks/pre-push"
  grep -Fq 'scripts/worktree-bootstrap.sh' "$REPO/.git/hooks/post-checkout"
  grep -Fq 'run "post-checkout"' "$REPO/.git/hooks/post-checkout"

  [ "$(yq -o=json -I=0 '."pre-commit".jobs' "$REPO/.git/lefthook/lefthook.yml")" = '[{"name":"worktree-guard","run":"scripts/worktree-guard.sh"}]' ]
  [ "$(yq -o=json -I=0 '."pre-push".jobs' "$REPO/.git/lefthook/lefthook.yml")" = '[{"name":"worktree-guard","run":"scripts/worktree-guard.sh"}]' ]
  [ "$(yq -o=json -I=0 '."post-checkout".jobs' "$REPO/.git/lefthook/lefthook.yml")" = '[{"name":"worktree-bootstrap","run":"scripts/worktree-bootstrap.sh"}]' ]
}

@test "ordinary main-checkout commits and pushes remain blocked" {
  install_repo

  run git -C "$REPO" commit -q --allow-empty -m blocked
  [ "$status" -ne 0 ]

  run git -C "$REPO" commit -q --allow-empty --no-verify -m seed-for-push
  [ "$status" -eq 0 ]

  run bash -c "cd '$REPO' && .git/hooks/pre-push origin example.invalid"
  [ "$status" -eq 1 ]
  [[ "$output" == *"main checkout"* ]]
}

@test "linked worktrees remain usable and post-checkout bootstraps them" {
  local linked="$REPO/linked"
  install_repo

  run git -C "$REPO" worktree add -q "$linked" -b linked-use
  [ "$status" -eq 0 ]
  [ -f "$linked/.env.worktree" ]
  [ -f "$linked/.envrc" ]

  run bash -c "cd '$linked' && '$REPO/.git/hooks/pre-push' origin example.invalid"
  [ "$status" -eq 0 ]
}

@test "backs up a foreign regular hook before atomically replacing it" {
  create_foreign_hook pre-commit

  run install_repo

  [ "$status" -eq 0 ]
  grep -qx 'echo foreign-pre-commit' "$REPO/.git/hooks/pre-commit.worktrees-backup"
  grep -Fq '# ai-plugins-managed-hook:v1:pre-commit' "$REPO/.git/hooks/pre-commit"
  [[ "$output" == *"hook_backup_created"* ]]
}

@test "preserves foreign symlink topology in the archival backup" {
  mkdir -p "$REPO/.git/hooks"
  printf '#!/usr/bin/env bash\necho linked-foreign\n' >"$REPO/foreign-hook"
  chmod +x "$REPO/foreign-hook"
  ln -s "$REPO/foreign-hook" "$REPO/.git/hooks/pre-commit"

  run install_repo

  [ "$status" -eq 0 ]
  [ -L "$REPO/.git/hooks/pre-commit.worktrees-backup" ]
  [ "$(readlink "$REPO/.git/hooks/pre-commit.worktrees-backup")" = "$REPO/foreign-hook" ]
  [ -f "$REPO/.git/hooks/pre-commit" ]
  [ ! -L "$REPO/.git/hooks/pre-commit" ]
}

@test "uses a unique suffix without overwriting an earlier foreign backup" {
  create_foreign_hook pre-commit
  printf 'first archive\n' >"$REPO/.git/hooks/pre-commit.worktrees-backup"

  run install_repo

  [ "$status" -eq 0 ]
  grep -qx 'first archive' "$REPO/.git/hooks/pre-commit.worktrees-backup"
  grep -qx 'echo foreign-pre-commit' "$REPO/.git/hooks/pre-commit.worktrees-backup.1"
}

@test "reinstalling managed hooks is idempotent and creates no backup" {
  install_repo
  local before
  before="$(hook_fingerprints)"

  run install_repo

  [ "$status" -eq 0 ]
  [ "$(hook_fingerprints)" = "$before" ]
  [ ! -e "$REPO/.git/hooks/pre-commit.worktrees-backup" ]
  [ ! -e "$REPO/.git/hooks/pre-push.worktrees-backup" ]
  [ ! -e "$REPO/.git/hooks/post-checkout.worktrees-backup" ]
}

@test "a concurrent installer is rejected and retry succeeds after the lock holder exits" {
  local lock="$REPO/.git/lefthook/install.lock"
  mkdir -p "$REPO/.git/lefthook"
  exec 8>"$lock"
  flock -n 8

  run install_repo
  [ "$status" -eq 75 ]
  [[ "$output" == *"hook_install_locked"* ]]
  [ ! -e "$REPO/.git/hooks/pre-commit" ]

  flock -u 8
  exec 8>&-

  run install_repo
  [ "$status" -eq 0 ]
}

@test "a surviving crashed-installer child keeps the lock until its process group exits" {
  local ready="$REPO/crash-ready"
  local real_nix_store
  local staged

  real_nix_store="$(command -v nix-store)"
  mkdir -p "$REPO/fake-bin"
  printf '#!/usr/bin/env bash\ncase " $* " in *" --add-root "*) touch "$AI_TEST_READY"; while :; do sleep 1; done ;; esac\nexec %q "$@"\n' \
    "$real_nix_store" >"$REPO/fake-bin/nix-store"
  chmod +x "$REPO/fake-bin/nix-store"

  setsid bash -c "cd '$REPO' && AI_TEST_READY='$ready' PATH='$REPO/fake-bin':\$PATH scripts/install-worktree-hooks.sh" \
    >"$REPO/crashed-installer.log" 2>&1 &
  CRASH_GROUP=$!
  for _ in $(seq 1 300); do
    [ -e "$ready" ] && break
    sleep 0.01
  done
  [ -e "$ready" ]
  staged="$(find "$REPO/.git/lefthook" -maxdepth 1 -type d -name '.install-staging.*' -print -quit)"
  [ -n "$staged" ]

  kill -KILL "$CRASH_GROUP"
  wait "$CRASH_GROUP" 2>/dev/null || true

  run install_repo

  [ "$status" -eq 75 ]
  [[ "$output" == *"hook_install_locked"* ]]
  [ -d "$staged" ]

  kill -KILL -- "-$CRASH_GROUP" 2>/dev/null || true
  CRASH_GROUP=""
  for _ in $(seq 1 300); do
    if flock --nonblock "$REPO/.git/lefthook/install.lock" true; then
      break
    fi
    sleep 0.01
  done

  run install_repo

  [ "$status" -eq 0 ]
  [[ "$output" == *"hook_staging_recovered"* ]]
  [ ! -e "$staged" ]
  for hook in pre-commit pre-push post-checkout; do
    grep -Fq "# ai-plugins-managed-hook:v1:$hook" "$REPO/.git/hooks/$hook"
  done

}

@test "ordinary local no_auto_install overrides cannot rewrite guarded launchers" {
  local before
  install_repo
  before="$(hook_fingerprints)"
  printf 'no_auto_install: false\n' >"$REPO/lefthook-local.yml"
  local head
  head="$(git -C "$REPO" rev-parse HEAD)"

  run bash -c "cd '$REPO' && .git/hooks/post-checkout '$head' '$head' 1"

  [ "$status" -eq 0 ]
  [ "$(hook_fingerprints)" = "$before" ]
}

@test "a backup-copy failure leaves the foreign hook active" {
  local real_cp
  create_foreign_hook pre-commit
  cp "$REPO/.git/hooks/pre-commit" "$REPO/original-hook"
  real_cp="$(command -v cp)"
  mkdir -p "$REPO/fake-bin"
  printf '#!/usr/bin/env bash\nlast="${!#}"\ncase "$last" in *.worktrees-backup*) exit 74 ;; esac\nexec %q "$@"\n' "$real_cp" >"$REPO/fake-bin/cp"
  chmod +x "$REPO/fake-bin/cp"

  run env PATH="$REPO/fake-bin:$PATH" bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 74 ]
  [[ "$output" == *"hook_install_incomplete"* ]]
  cmp "$REPO/original-hook" "$REPO/.git/hooks/pre-commit"
  [ ! -e "$REPO/.git/hooks/pre-commit.worktrees-backup" ]
}

@test "an atomic replacement failure leaves a valid mixed state and rerun converges" {
  local real_mv
  create_foreign_hook pre-push
  cp "$REPO/.git/hooks/pre-push" "$REPO/original-pre-push"
  real_mv="$(command -v mv)"
  mkdir -p "$REPO/fake-bin"
  printf '#!/usr/bin/env bash\nlast="${!#}"\ncase "$last" in */hooks/pre-push) case " $* " in *" --no-copy "*) exit 73 ;; *) printf "partial launcher\\n" >"$last"; exit 73 ;; esac ;; esac\nexec %q "$@"\n' \
    "$real_mv" >"$REPO/fake-bin/mv"
  chmod +x "$REPO/fake-bin/mv"

  run env PATH="$REPO/fake-bin:$PATH" bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 73 ]
  [[ "$output" == *"hook_install_incomplete"* ]]
  grep -Fq '# ai-plugins-managed-hook:v1:pre-commit' "$REPO/.git/hooks/pre-commit"
  cmp "$REPO/original-pre-push" "$REPO/.git/hooks/pre-push"
  grep -qx 'echo foreign-pre-push' "$REPO/.git/hooks/pre-push.worktrees-backup"

  run install_repo
  [ "$status" -eq 0 ]
  for hook in pre-commit pre-push post-checkout; do
    grep -Fq "# ai-plugins-managed-hook:v1:$hook" "$REPO/.git/hooks/$hook"
  done
}

@test "a Nix GC-root registration failure leaves active hooks untouched" {
  local real_nix_store
  create_foreign_hook pre-commit
  cp "$REPO/.git/hooks/pre-commit" "$REPO/original-hook"
  real_nix_store="$(command -v nix-store)"
  mkdir -p "$REPO/fake-bin"
  printf '#!/usr/bin/env bash\ncase " $* " in *" --add-root "*) exit 72 ;; esac\nexec %q "$@"\n' "$real_nix_store" >"$REPO/fake-bin/nix-store"
  chmod +x "$REPO/fake-bin/nix-store"

  run env PATH="$REPO/fake-bin:$PATH" bash -c "cd '$REPO' && scripts/install-worktree-hooks.sh"

  [ "$status" -eq 72 ]
  [[ "$output" == *"lefthook_gc_root_failed"* ]]
  cmp "$REPO/original-hook" "$REPO/.git/hooks/pre-commit"
  [ ! -e "$REPO/.git/hooks/pre-commit.worktrees-backup" ]
}

@test "unsupported hook target types fail before any hook is replaced" {
  mkdir -p "$REPO/.git/hooks"
  mkfifo "$REPO/.git/hooks/pre-push"

  run install_repo

  [ "$status" -eq 65 ]
  [[ "$output" == *"hook_target_unsupported"* ]]
  [ -p "$REPO/.git/hooks/pre-push" ]
  [ ! -e "$REPO/.git/hooks/pre-commit" ]
}

@test "installed launchers keep a pre-migration linked revision usable" {
  local linked="$REPO/pre-migration"
  local parent

  sed -i '/AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN/d' \
    "$REPO/scripts/worktree-guard.sh" \
    "$REPO/scripts/worktree-bootstrap.sh"
  git -C "$REPO" rm -q lefthook.yml
  git -C "$REPO" add scripts/worktree-guard.sh scripts/worktree-bootstrap.sh
  git -C "$REPO" commit -q -m 'test: record pre-migration revision'
  parent="$(git -C "$REPO" rev-parse HEAD)"

  cp "$BATS_TEST_DIRNAME/../worktree-guard.sh" "$REPO/scripts/worktree-guard.sh"
  cp "$BATS_TEST_DIRNAME/../worktree-bootstrap.sh" "$REPO/scripts/worktree-bootstrap.sh"
  cp "$BATS_TEST_DIRNAME/../../lefthook.yml" "$REPO/lefthook.yml"
  git -C "$REPO" add lefthook.yml scripts/worktree-guard.sh scripts/worktree-bootstrap.sh
  git -C "$REPO" commit -q -m 'test: restore migration revision'

  install_repo
  git -C "$REPO" worktree add -q --detach "$linked" "$parent"
  [ -f "$linked/.env.worktree" ]

  run git -C "$linked" commit -q --allow-empty -m 'test: commit from pre-migration revision'
  [ "$status" -eq 0 ]

  run bash -c "cd '$linked' && '$REPO/.git/hooks/pre-push' origin example.invalid"
  [ "$status" -eq 0 ]
}

@test "repository paths containing spaces and apostrophes are supported" {
  local moved="$BATS_TEST_TMPDIR/repo with spaces and 'apostrophe"
  mv "$REPO" "$moved"
  REPO="$moved"

  run install_repo

  [ "$status" -eq 0 ]
  run git -C "$REPO" commit -q --allow-empty -m blocked
  [ "$status" -ne 0 ]
}

@test "reinstalling after repository relocation repairs the indirect GC root" {
  local moved="$BATS_TEST_TMPDIR/relocated-repository"
  local new_root
  install_repo
  mv "$REPO" "$moved"
  REPO="$moved"

  run install_repo

  [ "$status" -eq 0 ]
  new_root="$(lefthook_root)"
  [ -L "$new_root" ]
  nix-store --query --roots "$AI_PLUGINS_LEFTHOOK_STORE_PATH" | grep -Fq "$new_root"
}
