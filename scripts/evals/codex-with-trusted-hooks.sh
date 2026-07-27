#!/usr/bin/env bash
set -euo pipefail

real_codex="${CODEX_EVAL_REAL_BIN:-}"
if [ -z "$real_codex" ] || [ ! -x "$real_codex" ]; then
  echo "CODEX_EVAL_REAL_BIN must name the executable pinned Codex CLI" >&2
  exit 2
fi
if [ "$#" -eq 0 ] || [ "$1" != "exec" ]; then
  echo "the trusted-hook eval wrapper supports only Codex exec" >&2
  exit 2
fi

shift
exec "$real_codex" exec --dangerously-bypass-hook-trust "$@"
