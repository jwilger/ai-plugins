#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"
manifest="$root/plugins/tiber/release-binaries.json"

if ! jq -e '
  (.binaries | type == "array") and
  ([.binaries[].target] | sort) == ([
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu"
  ] | sort) and
  all(.binaries[]; .path == ("dist/" + .target + "/tiber"))
' "$manifest" >/dev/null; then
  echo "invalid-release-manifest-shape path=$manifest" >&2
  exit 1
fi

# shellcheck source=/dev/null
source "$root/plugins/tiber/scripts/detect-target.sh"
host_target="$(detect_tiber_target)" || {
  echo "unsupported-host-release-binary os=$(uname -s) arch=$(uname -m)" >&2
  exit 1
}

host_path="$(
  jq -r --arg target "$host_target" \
    '.binaries[] | select(.target == $target) | .path' \
    "$manifest"
)"

if [ -z "$host_path" ] || [ ! -x "$root/plugins/tiber/$host_path" ]; then
  echo "missing-host-release-binary target=$host_target path=plugins/tiber/$host_path" >&2
  exit 1
fi

if [ ! -s "$root/plugins/tiber/$host_path" ]; then
  echo "invalid-host-release-binary target=$host_target path=plugins/tiber/$host_path" >&2
  exit 1
fi
