#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

release_inputs=(
  plugins/tiber/rust
  plugins/tiber/bin
  plugins/tiber/scripts
  plugins/tiber/release-binaries.json
  scripts/build-tiber-release-all.sh
  scripts/check-tiber-release-complete.sh
  scripts/check-tiber-release-fresh.sh
)
release_outputs=(
  plugins/tiber/dist
  plugins/tiber/release-binaries.sha256
)

source_clean=1
if ! git -C "$root" diff --quiet HEAD -- "${release_inputs[@]}"; then
  source_clean=0
fi

release_inputs_changed=1
release_outputs_changed=1
base_ref="${TIBER_RELEASE_FRESH_BASE:-origin/main}"
if git -C "$root" rev-parse --verify --quiet "$base_ref" >/dev/null; then
  merge_base="$(git -C "$root" merge-base HEAD "$base_ref")"
  if git -C "$root" diff --quiet "$merge_base" HEAD -- "${release_inputs[@]}"; then
    release_inputs_changed=0
  fi
  if git -C "$root" diff --quiet "$merge_base" HEAD -- "${release_outputs[@]}"; then
    release_outputs_changed=0
  fi
fi

if [ "$source_clean" -eq 0 ]; then
  if [ "${CI:-}" = "true" ]; then
    echo "dirty-release-inputs-in-ci" >&2
    git -C "$root" diff --stat HEAD -- "${release_inputs[@]}" >&2
    exit 1
  fi
  "$root/scripts/build-tiber-release-all.sh"
  "$root/scripts/check-tiber-release-complete.sh" "$root"
  echo "release-freshness-skip reason=dirty-release-inputs" >&2
  exit 0
fi

"$root/scripts/check-tiber-release-complete.sh" "$root"

if [ "$release_inputs_changed" -eq 0 ] && [ "$release_outputs_changed" -eq 0 ]; then
  echo "release-freshness-skip reason=no-release-input-changes" >&2
  exit 0
fi

"$root/scripts/build-tiber-release-all.sh"
"$root/scripts/check-tiber-release-complete.sh" "$root"

untracked_outputs="$(
  git -C "$root" ls-files --others --exclude-standard -- "${release_outputs[@]}"
)"
if [ -n "$untracked_outputs" ]; then
  echo "untracked-release-artifacts" >&2
  printf '%s\n' "$untracked_outputs" >&2
  exit 1
fi

if ! git -C "$root" diff --quiet -- "${release_outputs[@]}"; then
  echo "stale-release-artifacts reason=rebuild-changed-committed-outputs" >&2
  git -C "$root" diff --stat -- "${release_outputs[@]}" >&2
  exit 1
fi
