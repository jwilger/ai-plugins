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
allowed_evals='^(evals/|scripts/evals/|scripts/tests/evals-|site/evals/|\.github/workflows/(ci|live-evals|eval-pages)\.yml$|justfile$|package(-lock)?\.json$)'

git diff --name-only --cached >"${TMPDIR:-/tmp}/scope-cached.$$"
git diff --name-only >"${TMPDIR:-/tmp}/scope-worktree.$$"
trap 'rm -f "${TMPDIR:-/tmp}/scope-cached.$$" "${TMPDIR:-/tmp}/scope-worktree.$$"' EXIT

paths="$(cat "${TMPDIR:-/tmp}/scope-cached.$$" "${TMPDIR:-/tmp}/scope-worktree.$$" | sort -u)"

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
