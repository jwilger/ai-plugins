#!/usr/bin/env bash
set -euo pipefail
umask 077

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
caller_cwd="$(pwd -P)"
promptfoo_bin="${PROMPTFOO_BIN:-$root/node_modules/.bin/promptfoo}"
requested_out_dir="${EVAL_OUT_DIR:-$root/evals/out}"
case "$requested_out_dir" in
  /*) out_dir="$(realpath -m -- "$requested_out_dir")" ;;
  *) out_dir="$(realpath -m -- "$caller_cwd/$requested_out_dir")" ;;
esac
share_runtime="$(mktemp -d)"
log_file="$share_runtime/promptfoo-share.log"
context_marker_file="$share_runtime/context-leak-markers.json"
trap 'rm -rf -- "$share_runtime"' EXIT

node - "$root" "$context_marker_file" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const [root, output] = process.argv.slice(2);
const { hostContextLeakMarkers } = require(
  path.join(root, 'evals/promptfoo/fixtures.cjs'),
);
fs.writeFileSync(
  output,
  `${JSON.stringify({ version: 1, secrets: hostContextLeakMarkers() })}\n`,
  { mode: 0o600 },
);
fs.chmodSync(output, 0o600);
NODE

artifacts=()
for artifact in "$out_dir/results.json" "$out_dir/report.html" "$out_dir/results.junit.xml"; do
  [ ! -f "$artifact" ] || artifacts+=("$artifact")
done
if [ "${#artifacts[@]}" -eq 0 ]; then
  echo "promptfoo share requires scanned eval artifacts" >&2
  exit 2
fi

receipt="$out_dir/artifact-scan-receipt.json"
node "$root/scripts/evals/artifact-scan-receipt.mjs" verify \
  "$receipt" "${artifacts[@]}"

scan_args=(--private-root "$out_dir" --secret-file "$context_marker_file")
metadata="$out_dir/generated/agentic-systems-engineering.behavior.metadata.json"
if [ -f "$metadata" ]; then
  scan_args+=(--metadata "$metadata")
fi
"$root/scripts/evals/scan-behavior-artifacts.sh" \
  "${scan_args[@]}" -- "${artifacts[@]}"

if [ -z "${PROMPTFOO_BIN:-}" ]; then
  "$root/scripts/evals/ensure-node-deps.sh"
fi

set +e
"$promptfoo_bin" share "$@" >"$log_file" 2>&1
share_status="$?"
set -e

"$root/scripts/evals/scan-behavior-artifacts.sh" \
  --private-root "$share_runtime" \
  --secret-file "$context_marker_file" \
  -- "$log_file"

cat -- "$log_file"
[ "$share_status" -eq 0 ] || exit "$share_status"

share_url="$(grep -Eo "https?://[^[:space:])<>\\\"]+" "$log_file" | tail -n 1 || true)"
if [ -z "$share_url" ]; then
  echo "promptfoo share did not print a URL" >&2
  exit 1
fi

echo "Promptfoo share URL: $share_url"
