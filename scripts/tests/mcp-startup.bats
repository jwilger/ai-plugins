#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$BATS_TEST_TMPDIR"

  if [ ! -x "$ROOT/node_modules/.bin/promptfoo" ]; then
    "$ROOT/scripts/evals/ensure-node-deps.sh"
  fi

  MCP_TEST_PATH="$TMPROOT/mcp-test-path"
  mkdir -p "$MCP_TEST_PATH"
  ln -s "$(command -v node)" "$MCP_TEST_PATH/node"

  PROMPTFOO_FAKE_BIN="$TMPROOT/promptfoo-fake"
  printf '%s\n' \
    '#!/bin/sh' \
    'case "$PATH" in' \
    '  :*) echo "promptfoo.fake_leading_empty_path_segment PATH=$PATH" >&2; exit 42 ;;' \
    'esac' \
    'exit 0' >"$PROMPTFOO_FAKE_BIN"
  chmod +x "$PROMPTFOO_FAKE_BIN"
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

run_promptfoo_manifest_server_with_restricted_path() {
  install_promptfoo_cache_launcher
  run_manifest_server_with_restricted_path \
    "$ROOT/plugins/agentic-systems-engineering/.mcp.json" \
    promptfoo </dev/null
}

run_promptfoo_manifest_server_with_empty_path() {
  local command
  local args
  install_promptfoo_cache_launcher

  command="$(jq -r '.mcpServers.promptfoo.command' "$ROOT/plugins/agentic-systems-engineering/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.promptfoo.args[]' "$ROOT/plugins/agentic-systems-engineering/.mcp.json")

  env -i \
    PATH= \
    HOME="$HOME" \
    CODEX_HOME="$TMPROOT/codex-home" \
    PROMPTFOO_BIN="$PROMPTFOO_FAKE_BIN" \
    PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-state-empty-path" \
      "$command" "${args[@]}" </dev/null
}

run_promptfoo_manifest_server_from_fixture_repo() {
  local command
  local args
  install_promptfoo_cache_launcher

  command="$(jq -r '.mcpServers.promptfoo.command' "$ROOT/plugins/agentic-systems-engineering/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.promptfoo.args[]' "$ROOT/plugins/agentic-systems-engineering/.mcp.json")

  cd "$TMPROOT/repo"
  env -i \
    PATH="$MCP_TEST_PATH" \
    HOME="$HOME" \
    CODEX_HOME="$TMPROOT/codex-home" \
    PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-fixture-state" \
    "$command" "${args[@]}" </dev/null
}

run_tiber_manifest_server_with_restricted_path() {
  install_tiber_cache_launcher
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    run_manifest_server_with_restricted_path "$ROOT/plugins/tiber/.mcp.json" tiber
}

run_tiber_manifest_server_with_empty_path() {
  local command
  local args
  install_tiber_cache_launcher

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH= \
      HOME="$HOME" \
      CODEX_HOME="$TMPROOT/codex-home" \
      "$command" "${args[@]}"
}

run_tiber_manifest_server_with_untrusted_bash_first() {
  local command
  local args
  local untrusted_path="$TMPROOT/untrusted-path"
  install_tiber_cache_launcher

  mkdir -p "$untrusted_path"
  printf '%s\n' '#!/bin/sh' 'echo untrusted-bash-executed >&2' 'exit 42' >"$untrusted_path/bash"
  chmod +x "$untrusted_path/bash"

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$untrusted_path:/bin:/usr/bin:/run/current-system/sw/bin" \
      HOME="$HOME" \
      CODEX_HOME="$TMPROOT/codex-home" \
      "$command" "${args[@]}"
}

run_tiber_manifest_server_with_claude_plugin_root() {
  local command
  local args

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$MCP_TEST_PATH" \
      HOME="$HOME" \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/tiber" \
      "$command" "${args[@]}"
}

run_tiber_manifest_server_with_codex_home_and_claude_plugin_root() {
  local command
  local args
  local plugin_root="$TMPROOT/claude-plugin-root"

  install_tiber_cache_launcher
  mkdir -p "$plugin_root/bin"
  printf '%s\n' '#!/bin/sh' 'echo claude-plugin-root-used' 'exit 0' >"$plugin_root/bin/tiber"
  chmod +x "$plugin_root/bin/tiber"

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$MCP_TEST_PATH" \
      HOME="$HOME" \
      CODEX_HOME="$TMPROOT/codex-home" \
      CLAUDE_PLUGIN_ROOT="$plugin_root" \
      "$command" "${args[@]}"
}

run_tiber_manifest_server_with_invalid_claude_plugin_root() {
  local command
  local args
  local plugin_root="$TMPROOT/invalid-claude-plugin-root"

  install_tiber_cache_launcher
  mkdir -p "$plugin_root/bin"
  printf '%s\n' '#!/bin/sh' 'echo non-executable-claude-root-used' 'exit 0' >"$plugin_root/bin/tiber"

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  env -i \
    PATH="$MCP_TEST_PATH" \
    HOME="$HOME" \
    CODEX_HOME="$TMPROOT/codex-home" \
    CLAUDE_PLUGIN_ROOT="$plugin_root" \
    "$command" "${args[@]}"
}

run_tiber_manifest_server_with_missing_codex_cache() {
  local command
  local args
  local home_cache="$TMPROOT/home/.codex/plugins/cache/ai-plugins/tiber/0.7.0/bin"

  mkdir -p "$home_cache"
  printf '%s\n' '#!/bin/sh' 'echo home-codex-cache-used' 'exit 0' >"$home_cache/tiber"
  chmod +x "$home_cache/tiber"

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  env -i \
    PATH="$MCP_TEST_PATH" \
    HOME="$TMPROOT/home" \
    CODEX_HOME="$TMPROOT/missing-codex-home" \
    "$command" "${args[@]}"
}

run_tiber_manifest_server_with_default_home_codex_cache() {
  local command
  local args
  local home_cache="$TMPROOT/home/.codex/plugins/cache/ai-plugins/tiber"

  mkdir -p "$home_cache"
  ln -sfn "$ROOT/plugins/tiber" "$home_cache/0.7.0"

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$MCP_TEST_PATH" \
      HOME="$TMPROOT/home" \
      "$command" "${args[@]}"
}

run_tiber_manifest_server_without_home() {
  local command
  local args

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  env -i \
    PATH="$MCP_TEST_PATH" \
    "$command" "${args[@]}"
}

run_tiber_manifest_server_with_inherited_path_tooling() {
  local command
  local args
  local inherited_tool_path="$TMPROOT/inherited-tool-path"
  local plugin_root="$TMPROOT/fake-claude-plugin-root"

  mkdir -p "$inherited_tool_path" "$plugin_root/bin"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$inherited_tool_path/git"
  chmod +x "$inherited_tool_path/git"
  printf '%s\n' \
    '#!/bin/sh' \
    'case ":$PATH:" in' \
    "  *:$inherited_tool_path:*) echo inherited-path-preserved; exit 0 ;;" \
    '  *) echo inherited-path-missing PATH=$PATH >&2; exit 42 ;;' \
    'esac' >"$plugin_root/bin/tiber"
  chmod +x "$plugin_root/bin/tiber"

  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  env -i \
    PATH="$inherited_tool_path" \
    HOME="$HOME" \
    CLAUDE_PLUGIN_ROOT="$plugin_root" \
    "$command" "${args[@]}"
}

run_tiber_manifest_server_from_fixture_repo() {
  local command
  local args

  install_tiber_cache_launcher
  command="$(jq -r '.mcpServers.tiber.command' "$ROOT/plugins/tiber/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$ROOT/plugins/tiber/.mcp.json")

  cd "$TMPROOT/repo"
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$MCP_TEST_PATH" \
      HOME="$HOME" \
      CODEX_HOME="$TMPROOT/codex-home" \
      "$command" "${args[@]}"
}

install_tiber_cache_launcher() {
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/tiber"
  mkdir -p "$cache_parent"
  ln -sfn "$ROOT/plugins/tiber" "$cache_parent/0.7.0"
}

install_promptfoo_cache_launcher() {
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/agentic-systems-engineering"
  mkdir -p "$cache_parent"
  ln -sfn "$ROOT/plugins/agentic-systems-engineering" "$cache_parent/0.1.4"
}

install_stale_tiber_cache_launcher() {
  local cache_dir="$TMPROOT/codex-home/plugins/cache/ai-plugins/tiber/0.4.0/bin"
  mkdir -p "$cache_dir"
  printf '%s\n' '#!/bin/sh' 'echo stale-cache-tiber-executed >&2' 'exit 42' >"$cache_dir/tiber"
  chmod +x "$cache_dir/tiber"
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

  run run_promptfoo_manifest_server_with_restricted_path

  [ "$status" -eq 0 ]
  [[ "$output" != *"EROFS"* ]]
}

@test "promptfoo MCP manifest command starts without relying on PATH bash" {
  cd "$ROOT"

  run run_promptfoo_manifest_server_with_restricted_path

  [ "$status" -eq 0 ]
  [[ "$output" != *"No such file or directory"* ]]
  [[ "$output" != *"EROFS"* ]]
}

@test "promptfoo MCP manifest command does not create a leading empty PATH segment" {
  cd "$ROOT"

  run run_promptfoo_manifest_server_with_empty_path

  [ "$status" -eq 0 ]
  [[ "$output" != *"promptfoo.fake_leading_empty_path_segment"* ]]
}

@test "promptfoo MCP manifest command ignores repo-local launchers" {
  cd "$ROOT"

  mkdir -p "$TMPROOT/repo/bin" "$TMPROOT/repo/plugins/agentic-systems-engineering/bin"
  printf '%s\n' '#!/bin/sh' 'echo repo-local-promptfoo-executed >&2' 'exit 42' >"$TMPROOT/repo/bin/promptfoo-mcp"
  cp "$TMPROOT/repo/bin/promptfoo-mcp" "$TMPROOT/repo/plugins/agentic-systems-engineering/bin/promptfoo-mcp"
  chmod +x "$TMPROOT/repo/bin/promptfoo-mcp" "$TMPROOT/repo/plugins/agentic-systems-engineering/bin/promptfoo-mcp"

  run run_promptfoo_manifest_server_from_fixture_repo

  [ "$status" -eq 0 ]
  [[ "$output" != *"repo-local-promptfoo-executed"* ]]
}

@test "tiber MCP manifest command starts from the plugin cache" {
  cd "$ROOT/plugins/tiber"

  run run_tiber_manifest_server_with_restricted_path

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "tiber MCP manifest command ignores stale plugin cache versions" {
  cd "$ROOT/plugins/tiber"

  install_stale_tiber_cache_launcher
  run run_tiber_manifest_server_with_restricted_path

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
  [[ "$output" != *"stale-cache-tiber-executed"* ]]
}

@test "tiber MCP manifest command starts from Claude plugin root" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_claude_plugin_root

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "tiber MCP manifest command prefers Claude plugin root when present" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_codex_home_and_claude_plugin_root

  [ "$status" -eq 0 ]
  [[ "$output" == *"claude-plugin-root-used"* ]]
  [[ "$output" != *'"name":"tiber"'* ]]
}

@test "tiber MCP manifest command fails fast for invalid Claude plugin root" {
  cd "$ROOT"

  run -127 run_tiber_manifest_server_with_invalid_claude_plugin_root

  [ "$status" -eq 127 ]
  [[ "$output" == *"tiber.mcp_claude_plugin_root_invalid"* ]]
  [[ "$output" != *'"name":"tiber"'* ]]
}

@test "tiber MCP manifest command fails fast for missing explicit Codex cache" {
  cd "$ROOT"

  run -127 run_tiber_manifest_server_with_missing_codex_cache

  [ "$status" -eq 127 ]
  [[ "$output" == *"tiber.mcp_codex_cache_missing"* ]]
  [[ "$output" != *"home-codex-cache-used"* ]]
}

@test "tiber MCP manifest command starts from default home Codex cache" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_default_home_codex_cache

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "tiber MCP manifest command reports launcher missing when HOME is unset" {
  cd "$ROOT"

  run -127 run_tiber_manifest_server_without_home

  [ "$status" -eq 127 ]
  [[ "$output" == *"tiber.mcp_launcher_missing"* ]]
  [[ "$output" != *"unbound variable"* ]]
}

@test "tiber MCP manifest command preserves inherited PATH for subprocess tooling" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_inherited_path_tooling

  [ "$status" -eq 0 ]
  [[ "$output" == *"inherited-path-preserved"* ]]
}

@test "tiber MCP manifest command ignores repo-local launchers" {
  cd "$ROOT"

  mkdir -p "$TMPROOT/repo/bin" "$TMPROOT/repo/plugins/tiber/bin"
  printf '%s\n' '#!/bin/sh' 'echo repo-local-tiber-executed >&2' 'exit 42' >"$TMPROOT/repo/bin/tiber"
  cp "$TMPROOT/repo/bin/tiber" "$TMPROOT/repo/plugins/tiber/bin/tiber"
  chmod +x "$TMPROOT/repo/bin/tiber" "$TMPROOT/repo/plugins/tiber/bin/tiber"

  run run_tiber_manifest_server_from_fixture_repo

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
  [[ "$output" != *"repo-local-tiber-executed"* ]]
}

@test "tiber MCP manifest command starts without relying on PATH bash" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_restricted_path

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
  [[ "$output" != *"No such file or directory"* ]]
}

@test "tiber MCP manifest command starts with an empty PATH" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_empty_path

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
  [[ "$output" != *"No such file or directory"* ]]
}

@test "tiber MCP manifest command ignores untrusted PATH bash" {
  cd "$ROOT"

  run run_tiber_manifest_server_with_untrusted_bash_first

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
  [[ "$output" != *"untrusted-bash-executed"* ]]
}

@test "tiber MCP manifest command uses trusted Bash candidates for launcher scripts" {
  cd "$ROOT"

  run jq -r '.mcpServers.tiber.args[1]' "$ROOT/plugins/tiber/.mcp.json"

  [ "$status" -eq 0 ]
  [[ "$output" == *"for candidate_bash in /run/current-system/sw/bin/bash /bin/bash /usr/bin/bash"* ]]
  [[ "$output" == *'exec "$bash_bin" "$candidate" mcp stdio'* ]]
  [[ "$output" != *"command -v bash"* ]]
}
