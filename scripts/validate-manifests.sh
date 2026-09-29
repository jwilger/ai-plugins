#!/usr/bin/env bash
# Validate portable Agent Plugins manifests and the Codex marketplace index.
set -euo pipefail

root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"}"
marketplace="$root/.agents/plugins/marketplace.json"
fail() { printf 'manifest-sync: %s\n' "$*" >&2; exit 1; }
is_semver() { [[ "$1" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?(\+([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?$ ]]; }

[[ -f "$marketplace" ]] || fail "missing-marketplace: $marketplace"
jq empty "$marketplace" || fail "invalid-marketplace: $marketplace"
[[ $(jq -r '.plugins | length' "$marketplace") -eq $(jq -r '[.plugins[].name] | unique | length' "$marketplace") ]] || fail "duplicate-marketplace-plugin"

for dir in "$root"/plugins/*/; do
  [[ -d "$dir" ]] || continue
  name="$(basename "$dir")"
  manifest="${dir}plugin.json"
  [[ -f "$manifest" ]] || fail "missing-portable-plugin-json: $name"
  jq empty "$manifest" || fail "invalid-portable-plugin-json: $name"
  jq -e --arg name "$name" '
    ."$schema" == "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json" and
    .name == $name and
    (.description | type == "string") and
    (.extensions // {} | type == "object") and
    ([keys[]] - ["$schema", "name", "version", "description", "author", "homepage", "repository", "license", "keywords", "extensions"] | length == 0)
  ' "$manifest" >/dev/null || fail "invalid-portable-plugin-schema: $name"
  version="$(jq -r '.version // empty' "$manifest")"
  [[ -n "$version" ]] && is_semver "$version" || fail "invalid-plugin-version: $name version=$version"
  if [[ -f "$root/README.md" ]]; then
    catalog_version="$(awk -F '|' -v plugin="$name" 'index($2, "[" plugin "](") { gsub(/^[[:space:]]+|[[:space:]]+$/, "", $5); print $5; exit }' "$root/README.md")"
    [[ "$catalog_version" == "$version" ]] || fail "catalog-version-mismatch: $name catalog=$catalog_version plugin=$version"
  fi
  jq -e --arg name "$name" --arg version "$version" '
    [.plugins[] | select(.name == $name and .version == $version and
      .source.source == "local" and .source.path == ("./plugins/" + $name))] | length == 1
  ' "$marketplace" >/dev/null || fail "marketplace-plugin-mismatch: $name version=$version"
  [[ ! -e "${dir}.codex-plugin/plugin.json" ]] || fail "redundant-codex-plugin-json: $name"
  [[ ! -e "${dir}.codex-mcp.json" ]] || fail "legacy-codex-mcp-json: $name"
  if [[ -f "${dir}mcp.json" ]]; then
    jq -e '
      ."$schema" == "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json" and
      (.mcpServers | type == "object") and
      ([keys[]] - ["$schema", "mcpServers"] | length == 0) and
      ([.mcpServers[] | if .type == "stdio" then
        (.command | type == "string" and (startswith("./") or (contains("/") | not)))
      elif .type == "streamable-http" or .type == "sse" then
        (.url | type == "string")
      else false end
      ] | all)
    ' "${dir}mcp.json" >/dev/null || fail "invalid-portable-mcp-schema: $name"
    while IFS= read -r command; do
      [[ -f "${dir}${command#./}" ]] && [[ -x "${dir}${command#./}" ]] || fail "missing-mcp-launcher: $name command=$command"
    done < <(jq -r '.mcpServers[] | select(.type == "stdio" and (.command | startswith("./"))) | .command' "${dir}mcp.json")
  fi
done

while IFS= read -r name; do
  [[ -d "$root/plugins/$name" ]] || fail "marketplace-plugin-without-dir: $name"
done < <(jq -r '.plugins[].name' "$marketplace")

echo 'manifest-sync: ok'
