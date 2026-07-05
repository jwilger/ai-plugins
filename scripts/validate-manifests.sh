#!/usr/bin/env bash
# Validate that the Claude Code and Codex marketplace manifests stay in sync.
#
#   validate-manifests.sh [repo-root]
#
# Enforces (exit non-zero with a kebab-case reason on the first violation):
#   - both marketplace manifests exist;
#   - every Claude Code plugin is also available to Codex;
#   - Codex may carry additional Codex-only plugins;
#   - every name in either manifest has a matching plugins/<name>/ directory;
#   - every plugins/<name>/ directory is registered in at least one manifest;
#   - registered per-harness plugin manifests exist, use the directory name,
#     and carry valid semver versions;
#   - shared Claude Code + Codex plugin versions match.
set -euo pipefail

root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"}"
claude="$root/.claude-plugin/marketplace.json"
codex="$root/.agents/plugins/marketplace.json"

fail() {
  echo "manifest-sync: $*" >&2
  exit 1
}

is_semver() {
  [[ "$1" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?(\+([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?$ ]]
}

[ -f "$claude" ] || fail "missing-claude-manifest: $claude"
[ -f "$codex" ] || fail "missing-codex-manifest: $codex"

names_claude="$(jq -r '.plugins[].name' "$claude" | sort -u)"
names_codex="$(jq -r '.plugins[].name' "$codex" | sort -u)"
names_all="$(printf '%s\n%s\n' "$names_claude" "$names_codex" | sed '/^$/d' | sort -u)"

has_name() {
  local names="$1" name="$2"
  grep -qx "$name" <<<"$names"
}

# Claude Code has a built-in advisor mode, so repo plugins may be Codex-only,
# but this marketplace does not support Claude-only plugin entries.
while read -r name; do
  [ -n "$name" ] || continue
  has_name "$names_codex" "$name" || fail "claude-plugin-not-in-codex-marketplace: $name"
done <<<"$names_claude"

# Every registered name has a matching plugin directory.
while read -r name; do
  [ -n "$name" ] || continue
  [ -d "$root/plugins/$name" ] || fail "manifest-plugin-without-dir: $name"
done <<<"$names_all"

# Every plugin directory is registered in at least one manifest and carries the
# per-harness manifests required by its marketplace entries.
for dir in "$root"/plugins/*/; do
  [ -d "$dir" ] || continue
  name="$(basename "$dir")"
  in_claude=0
  in_codex=0
  has_name "$names_claude" "$name" && in_claude=1
  has_name "$names_codex" "$name" && in_codex=1

  [ "$in_claude" -eq 1 ] || [ "$in_codex" -eq 1 ] || fail "unregistered-plugin: $name"

  cc="${dir}.claude-plugin/plugin.json"
  cx="${dir}.codex-plugin/plugin.json"

  if [ "$in_claude" -eq 1 ]; then
    [ -f "$cc" ] || fail "missing-claude-plugin-json: $name"
  elif [ -e "$cc" ]; then
    fail "claude-plugin-json-without-marketplace: $name"
  fi

  if [ "$in_codex" -eq 1 ]; then
    [ -f "$cx" ] || fail "missing-codex-plugin-json: $name"
  fi

  if [ "$in_claude" -eq 1 ]; then
    cc_name="$(jq -r '.name' "$cc")"
    [ "$cc_name" = "$name" ] || fail "claude-plugin-name-mismatch: dir=$name json=$cc_name"
    cc_version="$(jq -r '.version // empty' "$cc")"
    [ -n "$cc_version" ] || fail "missing-claude-plugin-version: $name"
    is_semver "$cc_version" || fail "invalid-claude-plugin-version: $name version=$cc_version"

    marketplace_version="$(jq -r --arg name "$name" '.plugins[] | select(.name == $name) | .version // empty' "$claude")"
    [ -n "$marketplace_version" ] || fail "missing-claude-marketplace-version: $name"
    [ "$marketplace_version" = "$cc_version" ] || fail "claude-marketplace-version-mismatch: $name marketplace=$marketplace_version plugin=$cc_version"
  fi

  if [ "$in_codex" -eq 1 ]; then
    cx_name="$(jq -r '.name' "$cx")"
    [ "$cx_name" = "$name" ] || fail "codex-plugin-name-mismatch: dir=$name json=$cx_name"
    cx_version="$(jq -r '.version // empty' "$cx")"
    [ -n "$cx_version" ] || fail "missing-codex-plugin-version: $name"
    is_semver "$cx_version" || fail "invalid-codex-plugin-version: $name version=$cx_version"
  fi

  if [ "$in_claude" -eq 1 ] && [ "$in_codex" -eq 1 ]; then
    [ "$cc_version" = "$cx_version" ] || fail "plugin-version-mismatch: $name claude=$cc_version codex=$cx_version"
  fi
done

echo "manifest-sync: ok"
