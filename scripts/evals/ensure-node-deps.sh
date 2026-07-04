#!/usr/bin/env bash
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

required_paths=(
  "node_modules/.bin/promptfoo"
  "node_modules/@openai/codex-sdk"
  "node_modules/@anthropic-ai/claude-agent-sdk"
)

missing=0
for required_path in "${required_paths[@]}"; do
  if [ ! -e "$root/$required_path" ]; then
    missing=1
  fi
done

if [ "$missing" -eq 0 ]; then
  exit 0
fi

cd "$root"
npm ci --ignore-scripts --no-audit --no-fund
