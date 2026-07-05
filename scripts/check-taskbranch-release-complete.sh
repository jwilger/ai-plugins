#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"
manifest="$root/plugins/taskbranch/release-binaries.json"

"$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)/check-taskbranch-release-manifest.sh" "$root"

jq -r '.binaries[] | "\(.target)\t\(.path)"' "$manifest" |
  while IFS=$'\t' read -r target binary_path; do
    if [ ! -x "$root/plugins/taskbranch/$binary_path" ]; then
      echo "missing-release-binary target=$target path=plugins/taskbranch/$binary_path" >&2
      exit 1
    fi
    if [ ! -s "$root/plugins/taskbranch/$binary_path" ]; then
      echo "invalid-release-binary target=$target path=plugins/taskbranch/$binary_path" >&2
      exit 1
    fi
  done
