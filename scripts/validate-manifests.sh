#!/usr/bin/env bash
# Validate the Codex marketplace and its public plugin manifests.
set -euo pipefail

root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"}"
codex="$root/.agents/plugins/marketplace.json"

fail() { echo "manifest-sync: $*" >&2; exit 1; }
is_semver() { [[ "$1" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?(\+([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?$ ]]; }

[ -f "$codex" ] || fail "missing-codex-manifest: $codex"
jq empty "$codex" || fail "invalid-codex-manifest: $codex"
[ ! -e "$root/CLAUDE.md" ] || fail "unsupported-claude-instructions: CLAUDE.md"
[ ! -e "$root/.mcp.json" ] || fail "unsupported-claude-root-surface: .mcp.json"
root_claude_plugin="$(find "$root/.claude-plugin" -mindepth 1 -print -quit 2>/dev/null || true)"
[ -z "$root_claude_plugin" ] || fail "unsupported-claude-root-surface: ${root_claude_plugin#"$root/"}"
root_claude="$(find "$root/.claude" -mindepth 1 \( -path "$root/.claude/settings.local.json" -o -path "$root/.claude/worktrees" \) -prune -o -print -quit 2>/dev/null || true)"
[ -z "$root_claude" ] || fail "unsupported-claude-root-surface: ${root_claude#"$root/"}"
unsupported_claude="$(find "$root/plugins" \( -path '*/.claude-plugin/*' -o -name '.mcp.json' -o -path '*/hooks/hooks.json' -o -path '*/agents/*.md' -o -name 'CLAUDE.md' \) -print -quit)"
[ -z "$unsupported_claude" ] || fail "unsupported-claude-surface: ${unsupported_claude#"$root/"}"
names_codex="$(jq -r '.plugins[].name' "$codex" | sort -u)"

has_name() { grep -qx "$1" <<<"$names_codex"; }

while read -r name; do
  [ -n "$name" ] || continue
  [ -d "$root/plugins/$name" ] || fail "manifest-plugin-without-dir: $name"
  source_kind="$(jq -r --arg name "$name" '.plugins[] | select(.name == $name) | .source.source // empty' "$codex")"
  source_path="$(jq -r --arg name "$name" '.plugins[] | select(.name == $name) | .source.path // empty' "$codex")"
  [ "$source_kind" = "local" ] || fail "codex-marketplace-source-mismatch: $name source=$source_kind"
  [ "$source_path" = "./plugins/$name" ] || fail "codex-marketplace-path-mismatch: $name path=$source_path"
done <<<"$names_codex"

for dir in "$root"/plugins/*/; do
  [ -d "$dir" ] || continue
  name="$(basename "$dir")"
  has_name "$name" || fail "unregistered-plugin: $name"

  cx="${dir}.codex-plugin/plugin.json"
  [ -f "$cx" ] || fail "missing-codex-plugin-json: $name"
  jq empty "$cx" || fail "invalid-codex-plugin-json: $name"
  cx_name="$(jq -r '.name' "$cx")"
  [ "$cx_name" = "$name" ] || fail "codex-plugin-name-mismatch: dir=$name json=$cx_name"
  cx_version="$(jq -r '.version // empty' "$cx")"
  [ -n "$cx_version" ] || fail "missing-codex-plugin-version: $name"
  is_semver "$cx_version" || fail "invalid-codex-plugin-version: $name version=$cx_version"
  marketplace_version="$(jq -r --arg name "$name" '.plugins[] | select(.name == $name) | .version // empty' "$codex")"
  [ -n "$marketplace_version" ] || fail "missing-codex-marketplace-version: $name"
  [ "$marketplace_version" = "$cx_version" ] || fail "codex-marketplace-version-mismatch: $name marketplace=$marketplace_version plugin=$cx_version"

  mcp_ref="$(jq -r '.mcpServers // empty' "$cx")"
  codex_default_mcp="${dir}.codex-mcp.json"
  if [ -f "$codex_default_mcp" ] && [ -z "$mcp_ref" ]; then
    fail "codex-mcp-manifest-not-declared: $name path=./.codex-mcp.json"
  fi
  if [ -f "$codex_default_mcp" ] && [ -n "$mcp_ref" ] && [ "$mcp_ref" != "./.codex-mcp.json" ]; then
    fail "codex-mcp-manifest-not-declared: $name path=./.codex-mcp.json declared=$mcp_ref"
  fi
  if [ -n "$mcp_ref" ]; then
    mcp_path="${mcp_ref#./}"
    mcp_file="$dir$mcp_path"
    [ -f "$mcp_file" ] || fail "missing-codex-mcp-manifest: $name path=$mcp_ref"
    if jq -e '.mcpServers | to_entries[] | select(.value.command | type == "string" and startswith("./")) | select(.value.cwd != ".")' "$mcp_file" >/dev/null; then
      fail "codex-relative-mcp-command-requires-plugin-root-cwd: $name path=$mcp_ref"
    fi
    if grep -q "plugins/cache/" "$mcp_file"; then
      fail "codex-mcp-launcher-must-use-plugin-root: $name path=$mcp_ref"
    fi
  fi
done

echo "manifest-sync: ok"
