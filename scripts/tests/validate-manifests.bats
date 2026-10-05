#!/usr/bin/env bats

setup() {
  SCRIPT="$BATS_TEST_DIRNAME/../validate-manifests.sh"
  ROOT="$(mktemp -d)"
  mkdir -p "$ROOT/.agents/plugins" "$ROOT/plugins/alpha/bin"
  printf '#!/bin/sh\nexit 0\n' >"$ROOT/plugins/alpha/bin/alpha"
  chmod +x "$ROOT/plugins/alpha/bin/alpha"
  cat >"$ROOT/plugins/alpha/plugin.json" <<'JSON'
{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"alpha","version":"1.2.3","description":"Fixture"}
JSON
  cat >"$ROOT/.agents/plugins/marketplace.json" <<'JSON'
{"plugins":[{"name":"alpha","version":"1.2.3","source":{"source":"local","path":"./plugins/alpha"}}]}
JSON
}
teardown() { rm -rf "$ROOT"; }

@test "validates the real marketplace" {
  run bash "$SCRIPT"
  [ "$status" -eq 0 ]
}

@test "accepts a portable plugin without MCP" {
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "accepts a plugin-owned portable MCP launcher" {
  cat >"$ROOT/plugins/alpha/mcp.json" <<'JSON'
{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"alpha":{"type":"stdio","command":"./bin/alpha"}}}
JSON
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "rejects wrong or missing MCP schema" {
  printf '{"mcpServers":{"alpha":{"type":"stdio","command":"./bin/alpha"}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-portable-mcp-schema"* ]]
}

@test "rejects an MCP command outside its plugin" {
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"alpha":{"type":"stdio","command":"../bin/alpha"}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
}

@test "rejects a missing launcher" {
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"alpha":{"type":"stdio","command":"./bin/missing"}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-mcp-launcher"* ]]
}

@test "rejects mismatched versions" {
  jq '.version="1.2.4"' "$ROOT/plugins/alpha/plugin.json" >"$ROOT/next.json"
  mv "$ROOT/next.json" "$ROOT/plugins/alpha/plugin.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"marketplace-plugin-mismatch"* ]]
}

@test "rejects legacy duplicate Codex manifest" {
  mkdir -p "$ROOT/plugins/alpha/.codex-plugin"
  printf '{}\n' >"$ROOT/plugins/alpha/.codex-plugin/plugin.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"redundant-codex-plugin-json"* ]]
}

@test "accepts a pi package manifest that matches the plugin" {
  mkdir -p "$ROOT/plugins/alpha/skills" "$ROOT/plugins/alpha/pi"
  touch "$ROOT/plugins/alpha/pi/extension.ts"
  printf '%s\n' '{"name":"@example/alpha","version":"1.2.3","keywords":["pi-package"],"pi":{"extensions":["./pi/extension.ts"],"skills":["./skills"]}}' \
    >"$ROOT/plugins/alpha/package.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "rejects a pi package manifest whose version drifts from the plugin" {
  mkdir -p "$ROOT/plugins/alpha/skills"
  printf '%s\n' '{"name":"@example/alpha","version":"1.2.2","keywords":["pi-package"],"pi":{"skills":["./skills"]}}' \
    >"$ROOT/plugins/alpha/package.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"pi-package-version-mismatch: alpha"* ]]
}

@test "rejects a pi package resource path that does not exist" {
  printf '%s\n' '{"name":"@example/alpha","version":"1.2.3","keywords":["pi-package"],"pi":{"extensions":["./pi/missing.ts"]}}' \
    >"$ROOT/plugins/alpha/package.json"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-pi-resource: alpha path=./pi/missing.ts"* ]]
}
