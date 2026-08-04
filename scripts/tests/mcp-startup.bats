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

run_consolidated_manifest_server_from_cache() {
  local server="$1"
  local cache_layout="$2"
  local command
  local args
  local cache_parent
  local version

  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  case "$cache_layout" in
    codex-home)
      cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-system"
      ;;
    home)
      cache_parent="$TMPROOT/home/.codex/plugins/cache/ai-plugins/development-system"
      ;;
    *)
      echo "unknown consolidated MCP cache layout: $cache_layout" >&2
      return 2
      ;;
  esac
  mkdir -p "$cache_parent"
  ln -sfn "$ROOT/plugins/development-system" "$cache_parent/$version"

  command="$(jq -r ".mcpServers[\"$server\"].command" "$ROOT/plugins/development-system/.mcp.json")"
  mapfile -t args < <(jq -r ".mcpServers[\"$server\"].args[]" "$ROOT/plugins/development-system/.mcp.json")

  if [ "$cache_layout" = "codex-home" ]; then
    printf '%s\n' \
      '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' \
      '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' |
      env -i \
        PATH="$PATH" \
        HOME="$TMPROOT/home" \
        CODEX_HOME="$TMPROOT/codex-home" \
        CARGO_HOME="$ROOT/.dependencies/cargo" \
        "$command" "${args[@]}"
  else
    printf '%s\n' \
      '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' \
      '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' |
      env -i \
        PATH="$PATH" \
        HOME="$TMPROOT/home" \
        CARGO_HOME="$ROOT/.dependencies/cargo" \
        "$command" "${args[@]}"
  fi
}

run_promptfoo_manifest_server_with_restricted_path() {
  install_promptfoo_cache_launcher
  run_manifest_server_with_restricted_path \
    "$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json" \
    promptfoo </dev/null
}

run_promptfoo_manifest_server_with_empty_path() {
  local command
  local args
  install_promptfoo_cache_launcher

  command="$(jq -r '.mcpServers.promptfoo.command' "$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.promptfoo.args[]' "$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json")

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

  command="$(jq -r '.mcpServers.promptfoo.command' "$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers.promptfoo.args[]' "$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json")

  cd "$TMPROOT/repo"
  env -i \
    PATH="$MCP_TEST_PATH" \
    HOME="$HOME" \
    CODEX_HOME="$TMPROOT/codex-home" \
    PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-fixture-state" \
    "$command" "${args[@]}" </dev/null
}

run_development_discipline_manifest_server_with_claude_plugin_root() {
  local command
  local args

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      BASH_ENV="${BASH_ENV:-}" \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-system/components/development-discipline" \
      "$command" "${args[@]}"
}

run_development_discipline_manifest_server_with_codex_cache() {
  local command
  local args
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-system"
  local version

  mkdir -p "$cache_parent"
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-system" "$cache_parent/$version"

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")

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
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-system"
  local project_root="$TMPROOT/final-review-project"
  local version

  FINAL_REVIEW_FIXTURE_STATE_ROOT="$project_root/.development-discipline-state"

  mkdir -p "$cache_parent" "$project_root/.development-discipline"
  git -C "$project_root" init -q
  git -C "$project_root" config user.email final-review-fixture@example.test
  git -C "$project_root" config user.name 'Final Review Fixture'
  git -C "$project_root" config commit.gpgsign false
  git -C "$project_root" config core.hooksPath /dev/null
  git -C "$project_root" commit --allow-empty -qm 'initialize final-review fixture'
  mkdir -p "$project_root/src"
  printf '%s\n' 'fixture change' >"$project_root/src/new.rs"
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-system" "$cache_parent/$version"
  cat >"$project_root/.development-discipline/final-review.toml" <<'TOML'
[final_review.models]
pre_filter = "config-pre"
lens_review = "config-review"
post_filter = "config-post"
verifier = "config-verify"
TOML

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")

  env -i \
    PATH="$PATH" \
    HOME="$HOME" \
    CARGO_HOME="$ROOT/.dependencies/cargo" \
    CODEX_HOME="$TMPROOT/codex-home" \
    DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
    DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
    FINAL_REVIEW_TEST_PROJECT_ROOT="$project_root" \
    FINAL_REVIEW_ROUTING_PROJECT_ROOT="$ROOT" \
    node "$ROOT/scripts/tests/development-discipline-mcp-flow.mjs" \
    "$command" "${args[@]}"
}

run_development_discipline_manifest_server_with_both_harness_markers() {
  local command
  local args
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-system"
  local claude_root="$TMPROOT/claude-plugin-root"
  local version

  mkdir -p "$cache_parent" "$claude_root/bin"
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-system" "$cache_parent/$version"
  printf '%s\n' '#!/bin/sh' 'echo claude-plugin-root-used' >"$claude_root/bin/development-discipline-mcp"
  chmod +x "$claude_root/bin/development-discipline-mcp"

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")

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

  command="$(jq -r '.mcpServers["development-discipline"].command' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$ROOT/plugins/development-system/components/development-discipline/.mcp.json")

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$HOME" \
      CARGO_HOME="$ROOT/.dependencies/cargo" \
      CODEX_HOME="$TMPROOT/missing-codex-home" \
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-system/components/development-discipline" \
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
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-system/components/development-discipline" \
      "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"
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
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-system/components/development-discipline" \
      "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"
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
      CLAUDE_PLUGIN_ROOT="$ROOT/plugins/development-system/components/development-discipline" \
      "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"
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
    "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"
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
    "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"
}

run_development_discipline_cargo_fallback_without_home() {
  local bash_path
  bash_path="${BASH%/*}"

  env -i \
    PATH="$bash_path:/bin:/usr/bin" \
    DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1 \
    DEVELOPMENT_DISCIPLINE_MCP_FORCE_CARGO_FALLBACK=1 \
    "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"
}

install_promptfoo_cache_launcher() {
  local cache_parent="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-system"
  local version

  mkdir -p "$cache_parent"
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  ln -sfn "$ROOT/plugins/development-system" "$cache_parent/$version"
}

@test "development-discipline requires explicit source-build opt-in on Darwin" {
  local fake_uname="$TMPROOT/darwin-uname"
  local resolved_uname

  printf '%s\n' \
    '#!/bin/sh' \
    'case "$1" in' \
    '  -s) printf "%s\\n" Darwin ;;' \
    '  -m) printf "%s\\n" arm64 ;;' \
    '  *) exit 2 ;;' \
    'esac' >"$fake_uname"
  chmod +x "$fake_uname"
  resolved_uname="$(readlink -f /run/current-system/sw/bin/uname)"

  run "$AI_PLUGINS_BWRAP_BIN" \
    --ro-bind / / \
    --dev /dev \
    --proc /proc \
    --bind "$fake_uname" "$resolved_uname" \
    "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp"

  [ "$status" -eq 1 ]
  [ "$output" = "development-discipline.mcp.source_build_required target=aarch64-apple-darwin opt_in=DEVELOPMENT_DISCIPLINE_MCP_ALLOW_CARGO_FALLBACK=1" ]
}

@test "promptfoo MCP launcher starts with repo-local promptfoo and writable state" {
  cd "$ROOT/plugins/development-system/components/agentic-systems-engineering"

  run env PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-state" \
    timeout 20s ./bin/promptfoo-mcp </dev/null

  [ "$status" -eq 0 ]
  [[ "$output" != *"EROFS"* ]]
  [ -d "$TMPROOT/promptfoo-state/home" ]
  [ -d "$TMPROOT/promptfoo-state/config" ]
  [ -d "$TMPROOT/promptfoo-state/cache" ]
}

@test "consolidated development-discipline MCP starts from the explicit Codex cache without plugin-root variables" {
  cd "$ROOT"

  run run_consolidated_manifest_server_from_cache development-discipline codex-home

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" == *'"id":2'* ]]
  [[ "$output" == *'"name":"final_review.plan"'* ]]
  [[ "$output" != *"development_system.plugin_root_missing"* ]]
}

@test "consolidated development-discipline MCP starts from the HOME cache without plugin-root variables" {
  cd "$ROOT"

  run run_consolidated_manifest_server_from_cache development-discipline home

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" == *'"id":2'* ]]
  [[ "$output" == *'"name":"final_review.plan"'* ]]
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

  mkdir -p "$TMPROOT/repo/bin" "$TMPROOT/repo/plugins/development-system/components/agentic-systems-engineering/bin"
  printf '%s\n' '#!/bin/sh' 'echo repo-local-promptfoo-executed >&2' 'exit 42' >"$TMPROOT/repo/bin/promptfoo-mcp"
  cp "$TMPROOT/repo/bin/promptfoo-mcp" "$TMPROOT/repo/plugins/development-system/components/agentic-systems-engineering/bin/promptfoo-mcp"
  chmod +x "$TMPROOT/repo/bin/promptfoo-mcp" "$TMPROOT/repo/plugins/development-system/components/agentic-systems-engineering/bin/promptfoo-mcp"

  run run_promptfoo_manifest_server_from_fixture_repo

  [ "$status" -eq 0 ]
  [[ "$output" != *"repo-local-promptfoo-executed"* ]]
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
  [ "$(jq -r '.pre_filter' <<<"$routing")" = "strong-reviewer" ]
  [ "$(jq -r '.lens_review' <<<"$routing")" = "gpt-5.6-terra" ]
  [ "$(jq -r '.post_filter' <<<"$routing")" = "gpt-5.6-luna" ]
  [ "$(jq -r '.verifier' <<<"$routing")" = "strong-reviewer" ]
}

@test "development-discipline packaged MCP persists final-review sessions as Eventcore transactions" {
  cd "$ROOT"

  run run_development_discipline_codex_cache_final_review_flow

  [ "$status" -eq 0 ]
  [ -d "$TMPROOT/final-review-project/.development-discipline-state/development-discipline/final-review-sessions/events" ]
  run find "$TMPROOT/final-review-project/.development-discipline-state/development-discipline/final-review-sessions/events" -type f -name '*.jsonl'
  [ "$status" -eq 0 ]
  [ -n "$output" ]
  run rg 'ReportReplacedV1' "$TMPROOT/final-review-project/.development-discipline-state/development-discipline/final-review-sessions/events"
  [ "$status" -eq 0 ]
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
  [ "$output" = "$ROOT/plugins/development-system/components/development-discipline/rust" ]
}

@test "development-discipline MCP Cargo fallback ignores inherited target directory" {
  run run_development_discipline_cargo_fallback_with_untrusted_target_dir

  [ "$status" -eq 0 ]
  [ "$output" = "$ROOT/.dependencies/cargo-target/development-discipline" ]
}

@test "development-discipline MCP Cargo fallback handles unset HOME" {
  local untrusted_bin="$TMPROOT/home-unset-untrusted-bin"

  mkdir -p "$untrusted_bin"
  printf '%s\n' \
    '#!/bin/sh' \
    'echo "home-unset-untrusted-bash-executed" >&2' \
    'exit 99' >"$untrusted_bin/bash"
  chmod +x "$untrusted_bin/bash"
  PATH="$untrusted_bin:$PATH"

  run run_development_discipline_cargo_fallback_without_home

  [ "$status" -ne 0 ]
  [[ "$output" == *"development-discipline.mcp.missing_cargo"* ]]
  [[ "$output" != *"unbound variable"* ]]
  [[ "$output" != *"home-unset-untrusted-bash-executed"* ]]
}
