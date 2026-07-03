#!/usr/bin/env bash
# Validate that the Claude Code and Codex marketplace manifests stay in sync.
#
#   validate-manifests.sh [repo-root]
#
# Enforces (exit non-zero with a kebab-case reason on the first violation):
#   - both manifests exist and list the same set of plugin names;
#   - every name in the manifests has a matching plugins/<name>/ directory;
#   - every plugins/<name>/ directory is registered in both manifests;
#   - every plugin carries both .claude-plugin/plugin.json and
#     .codex-plugin/plugin.json, each whose `name` matches the directory;
#   - Claude Code and Codex plugin versions are valid semver and match.
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

if [ "$names_claude" != "$names_codex" ]; then
  fail "plugin-sets-differ: claude=[$(echo "$names_claude" | tr '\n' ' ')] codex=[$(echo "$names_codex" | tr '\n' ' ')]"
fi

# Every registered name has a matching plugin directory.
while read -r name; do
  [ -n "$name" ] || continue
  [ -d "$root/plugins/$name" ] || fail "manifest-plugin-without-dir: $name"
done <<<"$names_claude"

# Every plugin directory is registered in both manifests and carries matching
# per-harness manifests.
for dir in "$root"/plugins/*/; do
  [ -d "$dir" ] || continue
  name="$(basename "$dir")"

  echo "$names_claude" | grep -qx "$name" || fail "unregistered-plugin: $name"

  cc="${dir}.claude-plugin/plugin.json"
  cx="${dir}.codex-plugin/plugin.json"
  [ -f "$cc" ] || fail "missing-claude-plugin-json: $name"
  [ -f "$cx" ] || fail "missing-codex-plugin-json: $name"

  cc_name="$(jq -r '.name' "$cc")"
  cx_name="$(jq -r '.name' "$cx")"
  [ "$cc_name" = "$name" ] || fail "claude-plugin-name-mismatch: dir=$name json=$cc_name"
  [ "$cx_name" = "$name" ] || fail "codex-plugin-name-mismatch: dir=$name json=$cx_name"

  cc_version="$(jq -r '.version // empty' "$cc")"
  cx_version="$(jq -r '.version // empty' "$cx")"
  [ -n "$cc_version" ] || fail "missing-claude-plugin-version: $name"
  [ -n "$cx_version" ] || fail "missing-codex-plugin-version: $name"
  is_semver "$cc_version" || fail "invalid-claude-plugin-version: $name version=$cc_version"
  is_semver "$cx_version" || fail "invalid-codex-plugin-version: $name version=$cx_version"
  [ "$cc_version" = "$cx_version" ] || fail "plugin-version-mismatch: $name claude=$cc_version codex=$cx_version"

  marketplace_version="$(jq -r --arg name "$name" '.plugins[] | select(.name == $name) | .version // empty' "$claude")"
  [ -n "$marketplace_version" ] || fail "missing-claude-marketplace-version: $name"
  [ "$marketplace_version" = "$cc_version" ] || fail "claude-marketplace-version-mismatch: $name marketplace=$marketplace_version plugin=$cc_version"
done

echo "manifest-sync: ok"
