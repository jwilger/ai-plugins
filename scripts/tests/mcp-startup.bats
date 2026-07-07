#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$BATS_TEST_TMPDIR"

  if [ ! -x "$ROOT/node_modules/.bin/promptfoo" ]; then
    "$ROOT/scripts/evals/ensure-node-deps.sh"
  fi

  MCP_TEST_PATH="$TMPROOT/mcp-test-path"
  mkdir -p "$MCP_TEST_PATH"
  ln -s "$(command -v node)" "$MCP_TEST_PATH/node"
}

run_manifest_server_with_restricted_path() {
  local manifest="$1"
  local server="$2"
  local command
  local args

  command="$(jq -r ".mcpServers[\"$server\"].command" "$manifest")"
  mapfile -t args < <(jq -r ".mcpServers[\"$server\"].args[]" "$manifest")

  env -i \
    PATH="$MCP_TEST_PATH" \
    HOME="$HOME" \
    CODEX_HOME="$TMPROOT/codex-home" \
    PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-state" \
    "$command" "${args[@]}"
}

run_tiber_manifest_server_with_restricted_path() {
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    run_manifest_server_with_restricted_path "$ROOT/plugins/tiber/.mcp.json" tiber
}

@test "promptfoo MCP launcher starts with repo-local promptfoo and writable state" {
  cd "$ROOT/plugins/agentic-systems-engineering"

  run env PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-state" \
    timeout 5s ./bin/promptfoo-mcp

  [ "$status" -eq 0 ]
  [[ "$output" != *"EROFS"* ]]
  [ -d "$TMPROOT/promptfoo-state/home" ]
  [ -d "$TMPROOT/promptfoo-state/config" ]
  [ -d "$TMPROOT/promptfoo-state/cache" ]
}

@test "promptfoo MCP manifest command resolves from the marketplace root" {
  cd "$ROOT"

  run env PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-root-state" \
    timeout 5s bash -lc 'set -euo pipefail; for candidate in ./bin/promptfoo-mcp ./plugins/agentic-systems-engineering/bin/promptfoo-mcp "${CODEX_HOME:-$HOME/.codex}"/plugins/cache/ai-plugins/agentic-systems-engineering/*/bin/promptfoo-mcp; do if [ -x "$candidate" ]; then exec "$candidate"; fi; done; echo "promptfoo.mcp_launcher_missing" >&2; exit 127'

  [ "$status" -eq 0 ]
  [[ "$output" != *"EROFS"* ]]
}

@test "promptfoo MCP manifest command starts without relying on PATH bash" {
  cd "$ROOT"

  run run_manifest_server_with_restricted_path \
    "$ROOT/plugins/agentic-systems-engineering/.mcp.json" \
    promptfoo

  [ "$status" -eq 0 ]
  [[ "$output" != *"No such file or directory"* ]]
  [[ "$output" != *"EROFS"* ]]
}

@test "tiber MCP manifest command starts from the plugin root" {
  cd "$ROOT/plugins/tiber"

  run bash -c "printf '%s\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"bats\",\"version\":\"0.0.0\"}}}' | timeout 5s bash -lc 'set -euo pipefail; for candidate in ./bin/tiber ./plugins/tiber/bin/tiber \"\${CODEX_HOME:-\$HOME/.codex}\"/plugins/cache/ai-plugins/tiber/*/bin/tiber; do if [ -x \"\$candidate\" ]; then exec \"\$candidate\" mcp stdio; fi; done; echo \"tiber.mcp_launcher_missing\" >&2; exit 127'"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "tiber MCP manifest command resolves from the marketplace root" {
  cd "$ROOT"

  run bash -c "printf '%s\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"bats\",\"version\":\"0.0.0\"}}}' | timeout 5s bash -lc 'set -euo pipefail; for candidate in ./bin/tiber ./plugins/tiber/bin/tiber \"\${CODEX_HOME:-\$HOME/.codex}\"/plugins/cache/ai-plugins/tiber/*/bin/tiber; do if [ -x \"\$candidate\" ]; then exec \"\$candidate\" mcp stdio; fi; done; echo \"tiber.mcp_launcher_missing\" >&2; exit 127'"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "tiber MCP manifest command starts without relying on PATH bash" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_restricted_path

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
  [[ "$output" != *"No such file or directory"* ]]
}
