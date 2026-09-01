#!/usr/bin/env bats
# Tests for the Codex marketplace manifest validator.

setup() {
  SCRIPT="$BATS_TEST_DIRNAME/../validate-manifests.sh"
  ROOT="$(mktemp -d)"
}

teardown() { rm -rf "$ROOT"; }

make_plugin() {
  local name="$1" json_name="${2:-$1}" version="${3:-1.2.3}"
  mkdir -p "$ROOT/plugins/$name/.codex-plugin"
  printf '{"name":"%s","version":"%s"}\n' "$json_name" "$version" >"$ROOT/plugins/$name/.codex-plugin/plugin.json"
}

write_manifest() {
  mkdir -p "$ROOT/.agents/plugins"
  local entries="" name
  for name in $1; do
    entries="${entries}{\"name\":\"$name\",\"source\":{\"source\":\"local\",\"path\":\"./plugins/$name\"},\"version\":\"1.2.3\"},"
  done
  printf '{"plugins":[%s]}\n' "${entries%,}" >"$ROOT/.agents/plugins/marketplace.json"
}

@test "passes on the real repository" {
  run bash "$SCRIPT"
  [ "$status" -eq 0 ]
}

@test "passes a well-formed Codex marketplace" {
  make_plugin alpha
  make_plugin beta
  write_manifest "alpha beta"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "fails when a Claude marketplace manifest remains" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/.claude-plugin"
  printf '{"plugins":[]}\n' >"$ROOT/.claude-plugin/marketplace.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-root-surface"* ]]
}

@test "fails when another root Claude plugin surface remains" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/.claude-plugin"
  printf '{}\n' >"$ROOT/.claude-plugin/plugin.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-root-surface"* ]]
}

@test "fails when root Claude project configuration remains" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/.claude"
  printf '{}\n' >"$ROOT/.claude/settings.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-root-surface"* ]]
}

@test "fails when a root legacy MCP manifest remains" {
  make_plugin alpha
  write_manifest "alpha"
  printf '{}\n' >"$ROOT/.mcp.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-root-surface"* ]]
}

@test "allows ignored owner-local Claude leftovers" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/.claude/worktrees/example"
  printf '{}\n' >"$ROOT/.claude/settings.local.json"
  printf 'legacy\n' >"$ROOT/.claude/worktrees/example/state"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "fails when a Claude plugin manifest remains" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/plugins/alpha/.claude-plugin"
  printf '{"name":"alpha","version":"1.2.3"}\n' >"$ROOT/plugins/alpha/.claude-plugin/plugin.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-surface"* ]]
}

@test "fails when a nested component Claude plugin manifest remains" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/plugins/alpha/components/example/.claude-plugin"
  printf '{"name":"example","version":"1.2.3"}\n' >"$ROOT/plugins/alpha/components/example/.claude-plugin/plugin.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-surface"* ]]
}

@test "fails when a legacy component MCP manifest remains" {
  make_plugin alpha
  write_manifest "alpha"
  printf '{}\n' >"$ROOT/plugins/alpha/.mcp.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-surface"* ]]
}

@test "fails when legacy component hooks remain" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/plugins/alpha/hooks"
  printf '{}\n' >"$ROOT/plugins/alpha/hooks/hooks.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-surface"* ]]
}

@test "fails when a legacy Markdown agent remains" {
  make_plugin alpha
  write_manifest "alpha"
  mkdir -p "$ROOT/plugins/alpha/agents"
  printf '# legacy\n' >"$ROOT/plugins/alpha/agents/reviewer.md"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-surface"* ]]
}

@test "fails when component Claude instructions remain" {
  make_plugin alpha
  write_manifest "alpha"
  printf '# legacy\n' >"$ROOT/plugins/alpha/CLAUDE.md"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-surface"* ]]
}

@test "fails when root Claude instructions remain" {
  make_plugin alpha
  write_manifest "alpha"
  printf '# legacy\n' >"$ROOT/CLAUDE.md"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-claude-instructions"* ]]
}

@test "fails when a plugin directory is unregistered" {
  make_plugin alpha
  make_plugin beta
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unregistered-plugin"* ]]
}

@test "fails when the marketplace lists a plugin with no directory" {
  make_plugin alpha
  write_manifest "alpha ghost"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"manifest-plugin-without-dir"* ]]
}

@test "fails when a Codex marketplace source is not local" {
  make_plugin alpha
  write_manifest "alpha"
  jq '.plugins[0].source.source = "remote"' "$ROOT/.agents/plugins/marketplace.json" >"$ROOT/marketplace.tmp"
  mv "$ROOT/marketplace.tmp" "$ROOT/.agents/plugins/marketplace.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-marketplace-source-mismatch"* ]]
}

@test "fails when a Codex marketplace source path does not match its plugin" {
  make_plugin alpha
  write_manifest "alpha"
  jq '.plugins[0].source.path = "./plugins/wrong"' "$ROOT/.agents/plugins/marketplace.json" >"$ROOT/marketplace.tmp"
  mv "$ROOT/marketplace.tmp" "$ROOT/.agents/plugins/marketplace.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-marketplace-path-mismatch"* ]]
}

@test "fails when a Codex plugin manifest is missing" {
  make_plugin alpha
  rm "$ROOT/plugins/alpha/.codex-plugin/plugin.json"
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-codex-plugin-json"* ]]
}

@test "fails when a Codex plugin name mismatches its directory" {
  make_plugin alpha wrong-name
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-plugin-name-mismatch"* ]]
}

@test "fails when a Codex plugin version is not semver" {
  make_plugin alpha alpha not-semver
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-codex-plugin-version"* ]]
}

@test "fails when the marketplace version differs from the plugin version" {
  make_plugin alpha alpha 1.2.4
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-marketplace-version-mismatch"* ]]
}

@test "fails when a declared Codex MCP manifest is missing" {
  make_plugin alpha
  jq '.mcpServers="./.codex-mcp.json"' "$ROOT/plugins/alpha/.codex-plugin/plugin.json" >"$ROOT/plugins/alpha/.codex-plugin/plugin.json.tmp"
  mv "$ROOT/plugins/alpha/.codex-plugin/plugin.json.tmp" "$ROOT/plugins/alpha/.codex-plugin/plugin.json"
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-codex-mcp-manifest"* ]]
}

@test "fails when an undeclared default Codex MCP manifest exists" {
  make_plugin alpha
  printf '{"mcpServers":{}}\n' >"$ROOT/plugins/alpha/.codex-mcp.json"
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-mcp-manifest-not-declared"* ]]
}

@test "allows a plugin with no packaged MCP manifest" {
  make_plugin alpha
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "fails when a relative Codex MCP command has no plugin-root cwd" {
  make_plugin alpha
  printf '{"mcpServers":{"alpha":{"command":"./bin/alpha"}}}\n' >"$ROOT/plugins/alpha/.codex-mcp.json"
  jq '.mcpServers="./.codex-mcp.json"' "$ROOT/plugins/alpha/.codex-plugin/plugin.json" >"$ROOT/plugins/alpha/.codex-plugin/plugin.json.tmp"
  mv "$ROOT/plugins/alpha/.codex-plugin/plugin.json.tmp" "$ROOT/plugins/alpha/.codex-plugin/plugin.json"
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-relative-mcp-command-requires-plugin-root-cwd"* ]]
}

@test "fails when a Codex MCP launcher hard-codes a cache path" {
  make_plugin alpha
  printf '{"mcpServers":{"alpha":{"command":"/tmp/plugins/cache/alpha/bin/alpha"}}}\n' >"$ROOT/plugins/alpha/.codex-mcp.json"
  jq '.mcpServers="./.codex-mcp.json"' "$ROOT/plugins/alpha/.codex-plugin/plugin.json" >"$ROOT/plugins/alpha/.codex-plugin/plugin.json.tmp"
  mv "$ROOT/plugins/alpha/.codex-plugin/plugin.json.tmp" "$ROOT/plugins/alpha/.codex-plugin/plugin.json"
  write_manifest "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-mcp-launcher-must-use-plugin-root"* ]]
}
