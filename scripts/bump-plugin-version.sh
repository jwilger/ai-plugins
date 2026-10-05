#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
if [[ $# -ne 2 ]] || [[ ! -f "$root/plugins/$1/plugin.json" ]]; then
  echo 'usage: scripts/bump-plugin-version.sh <plugin> <patch|minor|major>' >&2
  exit 2
fi
name=$1
kind=$2
case "$kind" in patch|minor|major) ;; *) echo "invalid bump: $kind" >&2; exit 2 ;; esac
version="$(jq -er '.version' "$root/plugins/$name/plugin.json")"
[[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] || { echo "unsupported release version: $version" >&2; exit 1; }
major=${BASH_REMATCH[1]}
minor=${BASH_REMATCH[2]}
patch=${BASH_REMATCH[3]}
case "$kind" in
  major) version="$((major+1)).0.0" ;;
  minor) version="$major.$((minor+1)).0" ;;
  patch) version="$major.$minor.$((patch+1))" ;;
esac
jq --arg version "$version" '.version = $version' "$root/plugins/$name/plugin.json" >"$root/plugins/$name/plugin.json.tmp"
mv "$root/plugins/$name/plugin.json.tmp" "$root/plugins/$name/plugin.json"
if [[ -f "$root/plugins/$name/package.json" ]]; then
  jq --arg version "$version" '.version = $version' "$root/plugins/$name/package.json" >"$root/plugins/$name/package.json.tmp"
  mv "$root/plugins/$name/package.json.tmp" "$root/plugins/$name/package.json"
fi
jq --arg name "$name" --arg version "$version" '(.plugins[] | select(.name == $name) | .version) = $version' "$root/.agents/plugins/marketplace.json" >"$root/.agents/plugins/marketplace.json.tmp"
mv "$root/.agents/plugins/marketplace.json.tmp" "$root/.agents/plugins/marketplace.json"
if [[ -f "$root/README.md" ]] && grep -Fq "| [$name](" "$root/README.md"; then
  sed -E -i "s/(\| \[$name\]\([^|]+\|[^|]+\|[^|]+\| )[0-9]+\.[0-9]+\.[0-9]+( *\|)/\1$version\2/" "$root/README.md"
fi
printf '%s\n' "$name $version"
