#!/usr/bin/env bash
# Install shared git hooks for worktree bootstrap and main-checkout enforcement.
set -euo pipefail

repo="$(git rev-parse --show-toplevel)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
hooks_dir="$common_dir/hooks"

mkdir -p "$hooks_dir"

install_hook() {
  hook_name="$1"
  target="$hooks_dir/$hook_name"
  tmp="$(mktemp "$hooks_dir/$hook_name.XXXXXX")"
  cat >"$tmp"

  if [ -e "$target" ] && ! cmp -s "$tmp" "$target"; then
    cp -p "$target" "$target.worktrees-backup"
    printf 'backed up existing hook: %s\n' "$target.worktrees-backup" >&2
  fi

  mv "$tmp" "$target"
  chmod +x "$target"
}

install_hook pre-commit <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
exec "$(git rev-parse --show-toplevel)/scripts/worktree-guard.sh" "$@"
HOOK

install_hook pre-push <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
exec "$(git rev-parse --show-toplevel)/scripts/worktree-guard.sh" "$@"
HOOK

install_hook post-checkout <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
exec "$(git rev-parse --show-toplevel)/scripts/worktree-bootstrap.sh" "$@"
HOOK

printf 'installed worktree hooks for %s\n' "$repo"
