#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

release_inputs=(
  plugins/tiber/rust
  plugins/tiber/bin
  plugins/tiber/scripts
  plugins/tiber/release-binaries.json
)
release_outputs=(
  plugins/tiber/dist
  plugins/tiber/release-binaries.sha256
)

source_clean=1
if ! git -C "$root" diff --quiet HEAD -- "${release_inputs[@]}"; then
  source_clean=0
fi

"$root/scripts/build-tiber-release-all.sh"
"$root/scripts/check-tiber-release-complete.sh" "$root"

if [ "$source_clean" -eq 0 ]; then
  echo "release-freshness-skip reason=dirty-release-inputs" >&2
  exit 0
fi

if ! git -C "$root" diff --quiet -- "${release_outputs[@]}"; then
  echo "stale-release-artifacts reason=rebuild-changed-committed-outputs" >&2
  git -C "$root" diff --stat -- "${release_outputs[@]}" >&2
  exit 1
fi
