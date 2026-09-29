#!/usr/bin/env bats
setup() {
  SOURCE_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  ROOT="$(mktemp -d)"
  mkdir -p "$ROOT/plugins/alpha/bin"
  printf '#!/bin/sh\nexit 0\n' >"$ROOT/plugins/alpha/bin/server"
  chmod +x "$ROOT/plugins/alpha/bin/server"
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"alpha","version":"1.0.0"}\n' >"$ROOT/plugins/alpha/plugin.json"
}
teardown() { rm -rf "$ROOT"; }

@test "official schema accepts portable stdio and HTTPS entries" {
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"local":{"type":"stdio","command":"./bin/server"},"remote":{"type":"streamable-http","url":"https://example.org/mcp"}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run node "$SOURCE_ROOT/scripts/validate-portable-plugin.mjs" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "official schema rejects unknown MCP fields" {
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"local":{"type":"stdio","command":"./bin/server","env_vars":["SSH_AUTH_SOCK"]}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run node "$SOURCE_ROOT/scripts/validate-portable-plugin.mjs" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *'portable-schema: alpha'* ]]
}

@test "portable launcher cannot escape the package" {
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"local":{"type":"stdio","command":"./../outside"}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run node "$SOURCE_ROOT/scripts/validate-portable-plugin.mjs" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *'command escapes plugin root'* ]]
}

@test "portable launcher cannot follow a symlink outside the package" {
  ln -sf "$SOURCE_ROOT/install.sh" "$ROOT/plugins/alpha/bin/outside"
  printf '{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"local":{"type":"stdio","command":"./bin/outside"}}}\n' >"$ROOT/plugins/alpha/mcp.json"
  run node "$SOURCE_ROOT/scripts/validate-portable-plugin.mjs" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *'resolves outside plugin root'* ]]
}
