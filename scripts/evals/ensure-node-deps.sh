#!/usr/bin/env bash
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
tooling_root="$root/tooling/evals"
tooling_modules="$tooling_root/node_modules"
root_modules="$root/node_modules"
lock_fingerprint_file="$tooling_modules/.ai-plugins-eval-lock-fingerprint"
expected_lock_fingerprint="$(sha256sum "$tooling_root/package-lock.json" | awk '{print $1}')"

required_paths=(
  ".bin/promptfoo"
  "@openai/codex-sdk"
  "@anthropic-ai/claude-agent-sdk"
)

# Migrate the former root install without downloading it again. node_modules is
# disposable generated state; the stable root symlink preserves existing tool
# and module-resolution paths used throughout the eval harness.
if [ -d "$root_modules" ] && [ ! -L "$root_modules" ]; then
  if [ ! -e "$tooling_modules" ]; then
    mv "$root_modules" "$tooling_modules"
  else
    rm -rf -- "$root_modules"
  fi
fi

needs_restore=0
for required_path in "${required_paths[@]}"; do
  if [ ! -e "$tooling_modules/$required_path" ]; then
    needs_restore=1
  fi
done

if [ ! -f "$lock_fingerprint_file" ] ||
  [ "$(cat "$lock_fingerprint_file")" != "$expected_lock_fingerprint" ]; then
  needs_restore=1
fi

if [ "$needs_restore" -ne 0 ]; then
  npm --prefix "$tooling_root" ci --ignore-scripts --no-audit --no-fund
  for required_path in "${required_paths[@]}"; do
    if [ ! -e "$tooling_modules/$required_path" ]; then
      echo "eval dependency restore did not install required path: $required_path" >&2
      exit 2
    fi
  done
  printf '%s\n' "$expected_lock_fingerprint" >"$lock_fingerprint_file"
fi

if [ -L "$root_modules" ]; then
  rm -- "$root_modules"
elif [ -e "$root_modules" ]; then
  rm -rf -- "$root_modules"
fi
ln -s "tooling/evals/node_modules" "$root_modules"
