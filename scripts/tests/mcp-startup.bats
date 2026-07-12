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
  local home_cache="$TMPROOT/home/.codex/plugins/cache/ai-plugins/tiber/0.6.0/bin"

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
  local version

  mkdir -p "$home_cache"
  version="$(jq -r '.version' "$ROOT/plugins/tiber/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/tiber" "$home_cache/$version"

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

run_development_discipline_manifest_server_with_claude_plugin_root() {
  local command
  local args

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-discipline/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      BASH_ENV="${BASH_ENV:-}" \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-discipline" \
      "$command" "${args[@]}"
}

run_development_discipline_manifest_server_with_codex_cache() {
  local command
  local args
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-discipline"
  local version

  mkdir -p "$cache_parent"
  version="$(jq -r '.version' "$ROOT/plugins/development-discipline/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-discipline" "$cache_parent/$version"

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-discipline/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      CODEX_HOME="$TMPROOT/codex-home" \
      "$command" "${args[@]}"
}

run_development_discipline_codex_cache_final_review_flow() {
  local command
  local args
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-discipline"
  local project_root="$TMPROOT/final-review-project"
  local version

  mkdir -p "$cache_parent" "$project_root/.development-discipline"
  version="$(jq -r '.version' "$ROOT/plugins/development-discipline/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-discipline" "$cache_parent/$version"
  cat >"$project_root/.development-discipline/final-review.toml" <<'TOML'
[final_review.models]
pre_filter = "config-pre"
lens_review = "config-review"
post_filter = "config-post"
verifier = "config-verify"
TOML

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-discipline/.mcp.json")

  env -i \
    PATH="$PATH" \
    HOME="$HOME" \
    CARGO_HOME="$ROOT/.dependencies/cargo" \
    CODEX_HOME="$TMPROOT/codex-home" \
    FINAL_REVIEW_TEST_PROJECT_ROOT="$project_root" \
    FINAL_REVIEW_ROUTING_PROJECT_ROOT="$ROOT" \
    node "$ROOT/scripts/tests/development-discipline-mcp-flow.mjs" \
    "$command" "${args[@]}"
}

run_development_discipline_manifest_server_with_both_harness_markers() {
  local command
  local args
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-discipline"
  local claude_root="$TMPROOT/claude-plugin-root"
  local version

  mkdir -p "$cache_parent" "$claude_root/bin"
  version="$(jq -r '.version' "$ROOT/plugins/development-discipline/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-discipline" "$cache_parent/$version"
  printf '%s\n' '#!/bin/sh' 'echo claude-plugin-root-used' >"$claude_root/bin/development-discipline-mcp"
  chmod +x "$claude_root/bin/development-discipline-mcp"

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-discipline/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      CODEX_HOME="$TMPROOT/codex-home" \
      CLAUDE_PLUGIN_ROOT="$claude_root" \
      "$command" "${args[@]}"
}

run_development_discipline_manifest_server_with_missing_codex_cache_and_claude_plugin_root() {
  local command
  local args

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-discipline/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      CODEX_HOME="$TMPROOT/missing-codex-home" \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-discipline" \
      "$command" "${args[@]}"
}

run_development_discipline_manifest_server_with_untrusted_cargo_first() {
  local untrusted_path="$TMPROOT/untrusted-cargo-path"

  mkdir -p "$untrusted_path"
  printf '%s\n' '#!/bin/sh' 'echo untrusted-cargo-executed >&2' 'exit 42' >"$untrusted_path/cargo"
  chmod +x "$untrusted_path/cargo"

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$untrusted_path:/bin:/usr/bin:/run/current-system/sw/bin" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
      DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-discipline" \
      "$ROOT/plugins/development-discipline/bin/development-discipline-mcp"
}

run_development_discipline_manifest_server_with_untrusted_uname_first() {
  local untrusted_path="$TMPROOT/untrusted-uname-path"

  mkdir -p "$untrusted_path"
  printf '%s\n' \
    '#!/bin/sh' \
    'echo untrusted-uname-executed >&2' \
    'case "$1" in -s) echo Linux ;; -m) echo x86_64 ;; *) exit 42 ;; esac' \
    >"$untrusted_path/uname"
  chmod +x "$untrusted_path/uname"

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$untrusted_path:/bin:/usr/bin:/run/current-system/sw/bin" \
      HOME="$HOME" \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-discipline" \
      "$ROOT/plugins/development-discipline/bin/development-discipline-mcp"
}

run_development_discipline_manifest_server_with_untrusted_cargo_env() {
  local untrusted_path="$TMPROOT/untrusted-cargo-env"

  mkdir -p "$untrusted_path"
  printf '%s\n' '#!/bin/sh' 'echo untrusted-cargo-env-executed >&2' 'exit 42' >"$untrusted_path/cargo"
  chmod +x "$untrusted_path/cargo"

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO="$untrusted_path/cargo" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
      DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-discipline" \
      "$ROOT/plugins/development-discipline/bin/development-discipline-mcp"
}

run_development_discipline_cargo_fallback_from_reviewed_checkout() {
  local reviewed_checkout="$TMPROOT/reviewed-checkout"
  local fallback_home="$TMPROOT/fallback-home"
  local fake_cargo="$fallback_home/.cargo/bin/cargo"

  mkdir -p "$reviewed_checkout/.cargo" "$fallback_home/.cargo/bin"
  printf '%s\n' '#!/bin/sh' 'pwd -P' >"$fake_cargo"
  chmod +x "$fake_cargo"

  cd "$reviewed_checkout"
  env -i \
    PATH="/bin:/usr/bin:/run/current-system/sw/bin" \
    HOME="$fallback_home" \
    CARGO="$fake_cargo" \
    DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
    DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
    "$ROOT/plugins/development-discipline/bin/development-discipline-mcp"
}

run_development_discipline_cargo_fallback_with_untrusted_target_dir() {
  local fallback_home="$TMPROOT/target-dir-home"
  local fake_cargo="$fallback_home/.cargo/bin/cargo"

  mkdir -p "$fallback_home/.cargo/bin"
  printf '%s\n' '#!/bin/sh' 'printf "%s\n" "$CARGO_TARGET_DIR"' >"$fake_cargo"
  chmod +x "$fake_cargo"

  env -i \
    PATH="/bin:/usr/bin:/run/current-system/sw/bin" \
    HOME="$fallback_home" \
    CARGO="$fake_cargo" \
    CARGO_TARGET_DIR="$TMPROOT/poisoned-target" \
    DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
    DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
    "$ROOT/plugins/development-discipline/bin/development-discipline-mcp"
}

run_development_discipline_cargo_fallback_without_home() {
  env -i \
    PATH="/bin:/usr/bin" \
    DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
    DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
    "$ROOT/plugins/development-discipline/bin/development-discipline-mcp"
}

install_tiber_cache_launcher() {
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/tiber"
  local version

  mkdir -p "$cache_parent"
  version="$(jq -r '.version' "$ROOT/plugins/tiber/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/tiber" "$cache_parent/$version"
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
    timeout 20s ./bin/promptfoo-mcp </dev/null

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

@test "development-discipline MCP manifest command starts from Claude plugin root" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_claude_plugin_root

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "development-discipline MCP manifest clears inherited BASH_ENV before launcher startup" {
  local bash_env_file="$TMPROOT/malicious-bash-env"
  local marker="$TMPROOT/bash-env-executed"

  cd "$ROOT"
  printf 'touch %q\n' "$marker" >"$bash_env_file"
  export BASH_ENV="$bash_env_file"

  run run_development_discipline_manifest_server_with_claude_plugin_root
  unset BASH_ENV

  [ "$status" -eq 0 ]
  [ ! -e "$marker" ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
}

@test "development-discipline MCP manifest command starts from Codex plugin cache" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_codex_cache

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "development-discipline packaged MCP exposes final-review tools through Codex cache" {
  local routing

  cd "$ROOT"

  run run_development_discipline_codex_cache_final_review_flow

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"final_review.plan"'* ]]
  [[ "$output" == *'"protocolVersion":"2024-11-05"'* ]]
  [[ "$output" == *"bats-review:1:correctness-behavior"* ]]
  [[ "$output" == *"explicit-pre"* ]]
  [[ "$output" == *"config-review"* ]]
  [[ "$output" == *"project_toml_config"* ]]
  [[ "$output" == *"review_state_out_of_sync=true"* ]]
  [[ "$output" == *"review_session_complete=true"* ]]
  [[ "$output" == *"clean_streak"* ]]
  [[ "$output" == *"completed_iteration"* ]]
  routing="$(printf '%s\n' "$output" | jq -r 'select(.id == 12) | .result.content[0].text | fromjson | .model_roles')"
  [ "$(jq -r '.pre_filter' <<<"$routing")" = "gpt-5.6-luna" ]
  [ "$(jq -r '.lens_review' <<<"$routing")" = "gpt-5.6-terra" ]
  [ "$(jq -r '.post_filter' <<<"$routing")" = "gpt-5.6-luna" ]
  [ "$(jq -r '.verifier' <<<"$routing")" = "gpt-5.6-sol" ]
}

@test "development-discipline MCP manifest prefers Claude plugin root when both harness markers are present" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_both_harness_markers

  [ "$status" -eq 0 ]
  [[ "$output" == *"claude-plugin-root-used"* ]]
}

@test "development-discipline MCP manifest falls back to Claude plugin root when Codex cache is missing" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_missing_codex_cache_and_claude_plugin_root

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "development-discipline MCP launcher rejects untrusted PATH cargo" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_untrusted_cargo_first

  [ "$status" -ne 0 ]
  [[ "$output" == *"development-discipline.mcp.untrusted_cargo"* ]]
  [[ "$output" != *"untrusted-cargo-executed"* ]]
}

@test "development-discipline MCP launcher ignores untrusted PATH uname" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_untrusted_uname_first

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" != *"untrusted-uname-executed"* ]]
}

@test "development-discipline MCP launcher rejects untrusted CARGO env override" {
  cd "$ROOT"

  run run_development_discipline_manifest_server_with_untrusted_cargo_env

  [ "$status" -ne 0 ]
  [[ "$output" == *"development-discipline.mcp.untrusted_cargo"* ]]
  [[ "$output" != *"untrusted-cargo-env-executed"* ]]
}

@test "development-discipline MCP Cargo fallback ignores reviewed-checkout Cargo config" {
  run run_development_discipline_cargo_fallback_from_reviewed_checkout

  [ "$status" -eq 0 ]
  [ "$output" = "$ROOT/plugins/development-discipline/rust" ]
}

@test "development-discipline MCP Cargo fallback ignores inherited target directory" {
  run run_development_discipline_cargo_fallback_with_untrusted_target_dir

  [ "$status" -eq 0 ]
  [ "$output" = "$ROOT/.dependencies/cargo-target/development-discipline" ]
}

@test "development-discipline MCP Cargo fallback handles unset HOME" {
  run run_development_discipline_cargo_fallback_without_home

  [ "$status" -ne 0 ]
  [[ "$output" == *"development-discipline.mcp.missing_cargo"* ]]
  [[ "$output" != *"unbound variable"* ]]
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

@test "tiber MCP manifest forwards the SSH agent socket env var" {
  cd "$ROOT"

  run jq -e '.mcpServers.tiber.env_vars == ["SSH_AUTH_SOCK"]' "$ROOT/plugins/tiber/.mcp.json"

  [ "$status" -eq 0 ]
}
