#!/usr/bin/env bash
# Install shared git hooks for worktree bootstrap and main-checkout enforcement.
set -euo pipefail

repo="$(git rev-parse --show-toplevel)"
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
hooks_dir="$common_dir/hooks"

mkdir -p "$hooks_dir"

cat >"$hooks_dir/pre-commit" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
exec "$(git rev-parse --show-toplevel)/scripts/worktree-guard.sh" "$@"
HOOK

cat >"$hooks_dir/pre-push" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
exec "$(git rev-parse --show-toplevel)/scripts/worktree-guard.sh" "$@"
HOOK

cat >"$hooks_dir/post-checkout" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
exec "$(git rev-parse --show-toplevel)/scripts/worktree-bootstrap.sh" "$@"
HOOK

chmod +x "$hooks_dir/pre-commit" "$hooks_dir/pre-push" "$hooks_dir/post-checkout"

printf 'installed worktree hooks for %s\n' "$repo"
