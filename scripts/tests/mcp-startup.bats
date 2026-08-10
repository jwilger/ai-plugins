#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$BATS_TEST_TMPDIR"

  if [ ! -x "$ROOT/node_modules/.bin/promptfoo" ]; then
    "$ROOT/scripts/evals/ensure-node-deps.sh"
  fi
}

manifest_command() {
  local manifest=$1
  local server=$2
  local relative

  relative="$(jq -r ".mcpServers[\"$server\"].command" "$manifest")"
  realpath "$(dirname "$manifest")/$relative"
}

initialize_server() {
  local command=$1
  shift

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
    env -i \
      PATH="$PATH" \
      HOME="$TMPROOT/home" \
      CODEX_HOME="$TMPROOT/codex-home" \
      SSH_AUTH_SOCK="${SSH_AUTH_SOCK:-}" \
      "$command" "$@"
}

@test "component MCP manifests resolve only repository-owned launchers" {
  local promptfoo_manifest="$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json"
  local tiber_manifest="$ROOT/plugins/development-system/components/tiber/.mcp.json"
  local discipline_manifest="$ROOT/plugins/development-system/components/development-discipline/.mcp.json"

  [ "$(manifest_command "$promptfoo_manifest" promptfoo)" = \
    "$ROOT/plugins/development-system/components/agentic-systems-engineering/bin/promptfoo-mcp" ]
  [ "$(manifest_command "$tiber_manifest" tiber)" = \
    "$ROOT/plugins/development-system/components/tiber/bin/tiber" ]
  [ "$(manifest_command "$discipline_manifest" development-discipline)" = \
    "$ROOT/plugins/development-system/components/development-discipline/bin/development-discipline-mcp" ]
}

@test "promptfoo component manifest starts with repo-local dependencies and isolated state" {
  local manifest="$ROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json"
  local command
  command="$(manifest_command "$manifest" promptfoo)"

  run env \
    PROMPTFOO_BIN="$ROOT/node_modules/.bin/promptfoo" \
    PROMPTFOO_MCP_STATE_DIR="$TMPROOT/promptfoo-state" \
    "$command" </dev/null

  [ "$status" -eq 0 ]
  [[ "$output" != *"EROFS"* ]]
  [ -d "$TMPROOT/promptfoo-state/home" ]
  [ -d "$TMPROOT/promptfoo-state/config" ]
  [ -d "$TMPROOT/promptfoo-state/cache" ]
}

@test "tiber component manifest initializes without an installed marketplace cache" {
  local manifest="$ROOT/plugins/development-system/components/tiber/.mcp.json"
  local command
  local args
  command="$(manifest_command "$manifest" tiber)"
  mapfile -t args < <(jq -r '.mcpServers.tiber.args[]' "$manifest")

  run initialize_server "$command" "${args[@]}"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "development-discipline component manifest initializes the read-only plugin service" {
  local manifest="$ROOT/plugins/development-system/components/development-discipline/.mcp.json"
  local command
  local args
  command="$(manifest_command "$manifest" development-discipline)"
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$manifest")

  run initialize_server "$command" "${args[@]}"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" == *'"tools":{}'* ]]
}

@test "top-level MCP launchers ignore unrelated global marketplace state" {
  mkdir -p "$TMPROOT/home/.codex/plugins/cache/ai-plugins/development-system/0.0.0/bin"
  printf '%s\n' '#!/bin/sh' 'echo stale-global-launcher-used >&2' 'exit 42' \
    >"$TMPROOT/home/.codex/plugins/cache/ai-plugins/development-system/0.0.0/bin/tiber"
  chmod +x "$TMPROOT/home/.codex/plugins/cache/ai-plugins/development-system/0.0.0/bin/tiber"

  run initialize_server "$ROOT/plugins/development-system/bin/tiber" mcp stdio

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
  [[ "$output" != *"stale-global-launcher-used"* ]]
}

@test "packaged read-only development-discipline does not expose privileged final-review tools" {
  local command="$ROOT/plugins/development-system/bin/development-discipline-mcp"
  mkdir -p "$TMPROOT/codex-home"

  run initialize_server "$command" --service plugin-read-only

  [ "$status" -eq 0 ]
  [[ "$output" == *"Mutation services are intentionally unavailable"* ]]
  [[ "$output" != *'"name":"final_review.plan"'* ]]
  [[ "$output" != *'"name":"workflow.start"'* ]]
}
