#!/usr/bin/env bash
# Require a new SemVer for changed public plugin packages.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$root"
base="${1:-HEAD^}"
git cat-file -e "$base^{commit}" || { echo "version-check: invalid base $base" >&2; exit 2; }
for manifest in plugins/*/plugin.json; do
  [[ -f "$manifest" ]] || continue
  plugin="${manifest#plugins/}"
  plugin="${plugin%/plugin.json}"
  current="$(jq -er '.version' "$manifest")"
  previous="$(git show "$base:$manifest" 2>/dev/null | jq -r '.version // empty' || true)"
  [[ -n "$previous" ]] || continue
  changed=false
  paths=("plugins/$plugin" "scripts/build-$plugin-release.sh" "scripts/install-$plugin-binaries.sh")
  if [[ "$plugin" == development-system ]]; then
    paths+=(install.sh .github/workflows/release-development-system.yml)
  fi
  if [[ -n "$(git diff --name-only "$base" -- "${paths[@]}")" ]]; then
    changed=true
  fi
  if [[ "$changed" == true ]]; then
    [[ "$current" != "$previous" ]] || { echo "version-check: $plugin changed without a version bump ($current)" >&2; exit 1; }
    if [[ "$(printf '%s\n%s\n' "$previous" "$current" | sort -V | head -n 1)" != "$previous" ]]; then
      echo "version-check: $plugin version did not increase ($previous -> $current)" >&2
      exit 1
    fi
  fi
done
echo 'version-check: ok'
