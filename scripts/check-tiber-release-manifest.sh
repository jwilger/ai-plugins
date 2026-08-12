#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"
manifest="$root/plugins/development-system/components/tiber/release-binaries.json"

if ! jq -e '
  (.source_fingerprint | type == "string" and length == 64) and
  (.binaries | type == "array") and
  [.binaries[].target] == ["x86_64-unknown-linux-gnu"] and
  all(.binaries[]; .path == ("dist/" + .target + "/tiber"))
' "$manifest" >/dev/null; then
  echo "invalid-release-manifest-shape path=$manifest" >&2
  exit 1
fi

# shellcheck source=/dev/null
source "$root/plugins/development-system/components/tiber/scripts/detect-target.sh"
host_target="$(detect_tiber_target)" || {
  echo "unsupported-host-release-binary os=$(uname -s) arch=$(uname -m)" >&2
  exit 1
}

host_path="$(
  jq -r --arg target "$host_target" \
    '.binaries[] | select(.target == $target) | .path' \
    "$manifest"
)"

if [ -z "$host_path" ] || [ ! -x "$root/plugins/development-system/components/tiber/$host_path" ]; then
  echo "missing-host-release-binary target=$host_target path=plugins/development-system/components/tiber/$host_path" >&2
  exit 1
fi

if [ ! -s "$root/plugins/development-system/components/tiber/$host_path" ]; then
  echo "invalid-host-release-binary target=$host_target path=plugins/development-system/components/tiber/$host_path" >&2
  exit 1
fi
