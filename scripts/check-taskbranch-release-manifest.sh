#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"
manifest="$root/plugins/taskbranch/release-binaries.json"

jq -e '
  (.binaries | type == "array") and
  ([.binaries[].target] | sort) == ([
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu"
  ] | sort) and
  all(.binaries[]; .path == ("dist/" + .target + "/taskbranch"))
' "$manifest" >/dev/null

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) host_target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64 | Linux-arm64) host_target="aarch64-unknown-linux-gnu" ;;
  Darwin-x86_64) host_target="x86_64-apple-darwin" ;;
  Darwin-arm64 | Darwin-aarch64) host_target="aarch64-apple-darwin" ;;
  *)
    echo "unsupported-host-release-binary os=$(uname -s) arch=$(uname -m)" >&2
    exit 1
    ;;
esac

host_path="$(
  jq -r --arg target "$host_target" \
    '.binaries[] | select(.target == $target) | .path' \
    "$manifest"
)"

if [ -z "$host_path" ] || [ ! -x "$root/plugins/taskbranch/$host_path" ]; then
  echo "missing-host-release-binary target=$host_target path=plugins/taskbranch/$host_path" >&2
  exit 1
fi

if [ ! -s "$root/plugins/taskbranch/$host_path" ]; then
  echo "invalid-host-release-binary target=$host_target path=plugins/taskbranch/$host_path" >&2
  exit 1
fi
