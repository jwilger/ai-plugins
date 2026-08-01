#!/usr/bin/env bash
# Install the pinned Lefthook launcher for worktree bootstrap.
set -euo pipefail

readonly expected_lefthook_version="${AI_PLUGINS_LEFTHOOK_VERSION:-}"
readonly lefthook_bin="${AI_PLUGINS_LEFTHOOK_BIN:-}"
readonly lefthook_store_path="${AI_PLUGINS_LEFTHOOK_STORE_PATH:-}"

if [ -z "$lefthook_bin" ] || [ -z "$lefthook_store_path" ] || [ -z "$expected_lefthook_version" ]; then
  printf 'worktrees.lefthook_pinned_runtime_missing: run this command from the Nix devshell.\n' >&2
  exit 127
fi
if [ ! -x "$lefthook_bin" ] || [ ! -d "$lefthook_store_path" ]; then
  printf 'worktrees.lefthook_pinned_runtime_missing: the flake-selected Lefthook runtime is unavailable.\n' >&2
  exit 127
fi
if ! command -v flock >/dev/null 2>&1 || ! command -v nix-store >/dev/null 2>&1; then
  printf 'worktrees.hook_installer_tool_missing: the Nix devshell must provide flock and nix-store.\n' >&2
  exit 127
fi

actual_lefthook_version=""
version_status=0
actual_lefthook_version="$("$lefthook_bin" version)" || version_status=$?
if [ "$version_status" -ne 0 ]; then
  printf 'worktrees.lefthook_version_probe_failed: the pinned binary exited with status %s.\n' "$version_status" >&2
  exit "$version_status"
fi
if [ "$actual_lefthook_version" != "$expected_lefthook_version" ]; then
  printf 'worktrees.lefthook_version_mismatch: expected %s but found %s.\n' \
    "$expected_lefthook_version" "$actual_lefthook_version" >&2
  exit 65
fi

repo="$(git rev-parse --show-toplevel)"
git_dir="$(cd "$(git rev-parse --git-dir)" && pwd -P)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
if [ "$git_dir" != "$common_dir" ]; then
  printf 'worktrees.main_checkout_required: install shared hooks from the main checkout.\n' >&2
  exit 64
fi

hooks_path_status=0
configured_hooks_path="$(git config --get core.hooksPath)" || hooks_path_status=$?
if [ "$hooks_path_status" -eq 0 ]; then
  printf 'worktrees.core_hooks_path_configured: unset core.hooksPath before installing shared hooks (found %s).\n' \
    "$configured_hooks_path" >&2
  exit 65
fi
if [ "$hooks_path_status" -ne 1 ]; then
  printf 'worktrees.core_hooks_path_probe_failed: git config exited with status %s.\n' "$hooks_path_status" >&2
  exit "$hooks_path_status"
fi

if [ ! -f "$repo/lefthook.yml" ] || [ -L "$repo/lefthook.yml" ]; then
  printf 'worktrees.lefthook_config_missing: expected a regular file at %s.\n' "$repo/lefthook.yml" >&2
  exit 65
fi

config_status=0
LEFTHOOK=1 LEFTHOOK_CONFIG="$repo/lefthook.yml" "$lefthook_bin" validate >/dev/null || config_status=$?
if [ "$config_status" -ne 0 ]; then
  printf 'worktrees.lefthook_config_invalid: fix %s before installing hooks.\n' "$repo/lefthook.yml" >&2
  exit "$config_status"
fi

readonly hooks_dir="$common_dir/hooks"
readonly state_dir="$common_dir/lefthook"
readonly roots_dir="$state_dir/roots"
readonly lock_file="$state_dir/install.lock"
readonly root_name="lefthook-${lefthook_store_path##*/}"
readonly lefthook_root="$roots_dir/$root_name"

mkdir -p "$hooks_dir" "$state_dir"
if [ "${1:-}" != --ai-plugins-lock-held ]; then
  if ! flock --nonblock "$lock_file" true; then
    printf 'worktrees.hook_install_locked: another installer is active; retry after it exits.\n' >&2
    exit 75
  fi
  exec flock \
    --exclusive \
    --nonblock \
    --conflict-exit-code 75 \
    --no-fork \
    --verbose \
    "$lock_file" \
    "$0" --ai-plugins-lock-held "$@"
fi
shift

stage=""
cleanup() {
  local status=$?
  trap - EXIT
  set +e
  if [ -n "$stage" ] && [ -d "$stage" ]; then
    rm -rf -- "$stage"
  fi
  if [ "$status" -ne 0 ]; then
    printf '%s\n' \
      'worktrees.hook_install_incomplete: fix the reported error and rerun just worktree-hooks; the post-checkout launcher remains complete and foreign hooks are preserved.' >&2
  fi
  exit "$status"
}
trap cleanup EXIT

for abandoned in "$state_dir"/.install-staging.*; do
  [ -e "$abandoned" ] || continue
  rm -rf -- "$abandoned"
  printf 'worktrees.hook_staging_recovered: removed %s before retrying.\n' "$abandoned" >&2
done

stage="$(mktemp -d "$state_dir/.install-staging.XXXXXX")"

for hook_name in post-checkout; do
  target="$hooks_dir/$hook_name"
  if { [ -e "$target" ] || [ -L "$target" ]; } && [ ! -f "$target" ] && [ ! -L "$target" ]; then
    printf 'worktrees.hook_target_unsupported: %s must be a regular file or symlink.\n' "$target" >&2
    exit 65
  fi
done

mkdir -p "$roots_dir"
if { [ -e "$lefthook_root" ] || [ -L "$lefthook_root" ]; } && [ ! -L "$lefthook_root" ]; then
  printf 'worktrees.lefthook_gc_root_conflict: inspect and remove the non-symlink path %s, then retry.\n' \
    "$lefthook_root" >&2
  exit 65
fi
root_status=0
nix-store --add-root "$lefthook_root" -r "$lefthook_store_path" >/dev/null || root_status=$?
if [ "$root_status" -ne 0 ]; then
  printf 'worktrees.lefthook_gc_root_failed: could not register %s for %s.\n' \
    "$lefthook_root" "$lefthook_store_path" >&2
  exit "$root_status"
fi

cp "$repo/lefthook.yml" "$stage/lefthook.yml"
chmod 0444 "$stage/lefthook.yml"
snapshot_status=0
LEFTHOOK=1 LEFTHOOK_CONFIG="$stage/lefthook.yml" "$lefthook_bin" validate >/dev/null || snapshot_status=$?
if [ "$snapshot_status" -ne 0 ]; then
  printf 'worktrees.lefthook_config_invalid: the staged configuration could not be validated.\n' >&2
  exit "$snapshot_status"
fi

write_post_checkout_launcher() {
  local staged_hook="$stage/post-checkout"

  {
    printf '%s\n' '#!/usr/bin/env bash'
    printf '%s\n' '# ai-plugins-managed-hook:v1:post-checkout'
    printf '%s\n' 'set -euo pipefail'
    printf "LEFTHOOK_ROOT_NAME='%s'\n" "$root_name"
    printf '%s\n' "SAFETY_SCRIPT='scripts/worktree-bootstrap.sh'"
    printf '%s\n' "SAFETY_TOKEN='worktree-bootstrap'"
    printf '%s\n' 'REPO_ROOT=$(git rev-parse --show-toplevel)'
    printf '%s\n' 'COMMON_DIR=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)'
    printf '%s\n' 'LEFTHOOK_BIN="$COMMON_DIR/lefthook/roots/$LEFTHOOK_ROOT_NAME/bin/lefthook"'
    printf '%s\n' 'LEFTHOOK_CONFIG="$COMMON_DIR/lefthook/lefthook.yml"'
    printf '%s\n' 'unset AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN'
    printf '%s\n' '"$REPO_ROOT/$SAFETY_SCRIPT" "$@"'
    printf '%s\n' 'AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN="$SAFETY_TOKEN"'
    printf '%s\n' 'export AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN LEFTHOOK_CONFIG'
    printf '%s\n' 'exec "$LEFTHOOK_BIN" run "post-checkout" --no-auto-install "$@"'
  } >"$staged_hook"
  chmod 0755 "$staged_hook"
  bash -n "$staged_hook"
}

write_post_checkout_launcher

mv -fT -- "$stage/lefthook.yml" "$state_dir/lefthook.yml"

is_managed_post_checkout_hook() {
  local target="$1"
  local marker

  [ -f "$target" ] || return 1
  marker="$(sed -n '2p' "$target")" || return 1
  [ "$marker" = '# ai-plugins-managed-hook:v1:post-checkout' ]
}

is_exact_legacy_managed_hook() {
  local hook_name="$1"
  local target="$2"
  local root_line
  local expected_exec

  [ -f "$target" ] && [ ! -L "$target" ] || return 1
  [ "$(wc -l <"$target")" -eq 15 ] || return 1
  [ "$(sed -n '1p' "$target")" = '#!/usr/bin/env bash' ] || return 1
  [ "$(sed -n '2p' "$target")" = "# ai-plugins-managed-hook:v1:$hook_name" ] || return 1
  [ "$(sed -n '3p' "$target")" = 'set -euo pipefail' ] || return 1
  root_line="$(sed -n '4p' "$target")"
  [[ "$root_line" =~ ^LEFTHOOK_ROOT_NAME=\'[^\']+\'$ ]] || return 1
  [ "$(sed -n '5p' "$target")" = "SAFETY_SCRIPT='scripts/worktree-guard.sh'" ] || return 1
  [ "$(sed -n '6p' "$target")" = "SAFETY_TOKEN='worktree-guard'" ] || return 1
  [ "$(sed -n '7p' "$target")" = 'REPO_ROOT=$(git rev-parse --show-toplevel)' ] || return 1
  [ "$(sed -n '8p' "$target")" = 'COMMON_DIR=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)' ] || return 1
  [ "$(sed -n '9p' "$target")" = 'LEFTHOOK_BIN="$COMMON_DIR/lefthook/roots/$LEFTHOOK_ROOT_NAME/bin/lefthook"' ] || return 1
  [ "$(sed -n '10p' "$target")" = 'LEFTHOOK_CONFIG="$COMMON_DIR/lefthook/lefthook.yml"' ] || return 1
  [ "$(sed -n '11p' "$target")" = 'unset AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN' ] || return 1
  [ "$(sed -n '12p' "$target")" = '"$REPO_ROOT/$SAFETY_SCRIPT" "$@"' ] || return 1
  [ "$(sed -n '13p' "$target")" = 'AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN="$SAFETY_TOKEN"' ] || return 1
  [ "$(sed -n '14p' "$target")" = 'export AI_PLUGINS_REQUIRED_HOOK_ALREADY_RAN LEFTHOOK_CONFIG' ] || return 1
  printf -v expected_exec 'exec "$LEFTHOOK_BIN" run "%s" --no-auto-install "$@"' "$hook_name"
  [ "$(sed -n '15p' "$target")" = "$expected_exec" ]
}

next_backup_path() {
  local target="$1"
  local backup="$target.worktrees-backup"
  local suffix=0

  while [ -e "$backup" ] || [ -L "$backup" ]; do
    suffix=$((suffix + 1))
    backup="$target.worktrees-backup.$suffix"
  done
  printf '%s\n' "$backup"
}

target="$hooks_dir/post-checkout"
if [ -e "$target" ] || [ -L "$target" ]; then
  if ! is_managed_post_checkout_hook "$target"; then
    backup="$(next_backup_path "$target")"
    cp -a -T -- "$target" "$backup"
    printf 'worktrees.hook_backup_created: %s\n' "$backup" >&2
  fi
fi
mv --no-copy -fT -- "$stage/post-checkout" "$target"

rm -rf -- "$stage"
stage=""

for hook_name in pre-commit pre-push; do
  target="$hooks_dir/$hook_name"
  if is_exact_legacy_managed_hook "$hook_name" "$target"; then
    rm -- "$target"
    printf 'worktrees.legacy_managed_hook_removed: %s\n' "$target" >&2
  fi
done

trap - EXIT
printf 'installed Lefthook post-checkout worktree bootstrap hook for %s using %s\n' "$repo" "$expected_lefthook_version"
