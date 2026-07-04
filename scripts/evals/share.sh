#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
promptfoo_bin="${PROMPTFOO_BIN:-$root/node_modules/.bin/promptfoo}"
log_file="$(mktemp)"
trap 'rm -f "$log_file"' EXIT

if [ -z "${PROMPTFOO_BIN:-}" ]; then
  "$root/scripts/evals/ensure-node-deps.sh"
fi

"$promptfoo_bin" share "$@" 2>&1 | tee "$log_file"

share_url="$(grep -Eo "https?://[^[:space:])<>\\\"]+" "$log_file" | tail -n 1 || true)"
if [ -z "$share_url" ]; then
  echo "promptfoo share did not print a URL" >&2
  exit 1
fi

echo "Promptfoo share URL: $share_url"
