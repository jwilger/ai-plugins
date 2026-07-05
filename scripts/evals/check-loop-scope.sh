#!/usr/bin/env bash
set -euo pipefail

mode="${1:-}"

usage() {
  cat <<'USAGE'
Usage: scripts/evals/check-loop-scope.sh improve-plugins|improve-evals

Fails if the current git diff touches paths outside the requested improvement loop.
USAGE
}

case "$mode" in
  improve-plugins | improve-evals) ;;
  *)
    usage >&2
    exit 2
    ;;
esac

allowed_plugins='^plugins/[^/]+/(skills/[^/]+/(SKILL\.md|references/.*)|README\.md|\.claude-plugin/plugin\.json|\.codex-plugin/plugin\.json)$'
allowed_evals='^(evals/|scripts/evals/|scripts/tests/evals-|site/evals/|\.github/workflows/(ci|live-evals)\.yml$|justfile$|package(-lock)?\.json$)'

cached_file="$(mktemp)"
worktree_file="$(mktemp)"
untracked_file="$(mktemp)"
trap 'rm -f "$cached_file" "$worktree_file" "$untracked_file"' EXIT

git diff --name-only --cached >"$cached_file"
git diff --name-only >"$worktree_file"
git ls-files --others --exclude-standard >"$untracked_file"

paths="$(cat "$cached_file" "$worktree_file" "$untracked_file" | sort -u)"

if [ -z "$paths" ]; then
  exit 0
fi

while IFS= read -r path; do
  [ -n "$path" ] || continue
  case "$mode" in
    improve-plugins)
      if ! [[ "$path" =~ $allowed_plugins ]]; then
        echo "disallowed path for improve-plugins: $path" >&2
        exit 1
      fi
      ;;
    improve-evals)
      if [[ "$path" == plugins/* ]] || ! [[ "$path" =~ $allowed_evals ]]; then
        echo "disallowed path for improve-evals: $path" >&2
        exit 1
      fi
      ;;
  esac
done <<<"$paths"
