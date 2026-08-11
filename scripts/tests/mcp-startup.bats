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

list_server_tools() {
  local command=$1
  shift

  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' |
    env -i \
      PATH="$PATH" \
      HOME="$TMPROOT/home" \
      CODEX_HOME="$TMPROOT/codex-home" \
      SSH_AUTH_SOCK="${SSH_AUTH_SOCK:-}" \
      "$command" "$@"
}

initialize_codex_plugin_server() {
  local manifest=$1
  local server=$2
  local version
  local installed_root
  local command
  local cwd
  local args
  local env_args
  local env_var

  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  installed_root="$TMPROOT/codex-home/plugins/cache/ai-plugins/development-system/$version"
  mkdir -p "$(dirname "$installed_root")" "$TMPROOT/caller"
  ln -sfn "$ROOT/plugins/development-system" "$installed_root"
  command="$(jq -r ".mcpServers[\"$server\"].command" "$manifest")"
  cwd="$(jq -r ".mcpServers[\"$server\"].cwd // empty" "$manifest")"
  mapfile -t args < <(jq -r ".mcpServers[\"$server\"].args[]" "$manifest")
  env_args=(
    "PATH=$PATH"
    "HOME=$TMPROOT/home"
    "CODEX_HOME=$TMPROOT/codex-home"
  )
  while IFS= read -r env_var; do
    if [ -n "$env_var" ] && [ -n "${!env_var+x}" ]; then
      env_args+=("$env_var=${!env_var}")
    fi
  done < <(jq -r ".mcpServers[\"$server\"].env_vars[]?" "$manifest")

  (
    if [ "$cwd" = "." ]; then
      cd "$installed_root"
    else
      cd "$TMPROOT/caller"
    fi
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.0"}}}' |
      env -i \
        "${env_args[@]}" \
        "$command" "${args[@]}"
  )
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

@test "development-discipline component manifest initializes the advisory plugin service" {
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

@test "Codex plugin MCP manifest starts every server from an arbitrary caller directory" {
  local manifest="$ROOT/plugins/development-system/.codex-mcp.json"

  [ "$(jq -r '.mcpServers["development-discipline"].command' "$manifest")" = \
    "./bin/development-discipline-mcp" ]
  [ "$(jq -r '.mcpServers["development-discipline"].cwd' "$manifest")" = "." ]
  [ "$(jq -c '.mcpServers["development-discipline"].env_vars' "$manifest")" = \
    '["SSH_AUTH_SOCK"]' ]
  [ "$(jq -r '.mcpServers.tiber.command' "$manifest")" = "./bin/tiber" ]
  [ "$(jq -r '.mcpServers.tiber.cwd' "$manifest")" = "." ]
  [ "$(jq -c '.mcpServers.tiber.env_vars' "$manifest")" = \
    '["SSH_AUTH_SOCK"]' ]

  run initialize_codex_plugin_server "$manifest" development-discipline
  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]

  run initialize_codex_plugin_server "$manifest" tiber
  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
}

@test "Claude plugin MCP manifest starts the advisory coordination service" {
  local manifest="$ROOT/plugins/development-system/.mcp.json"
  local command="$ROOT/plugins/development-system/bin/development-discipline-mcp"
  local args
  mapfile -t args < <(jq -r '.mcpServers["development-discipline"].args[]' "$manifest")

  run list_server_tools "$command" "${args[@]}"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"final_review.plan"'* ]]
  [[ "$output" != *'"name":"workflow.start"'* ]]
}

@test "packaged advisory development-discipline exposes review coordination without project mutation tools" {
  local command="$ROOT/plugins/development-system/bin/development-discipline-mcp"
  mkdir -p "$TMPROOT/codex-home"

  run list_server_tools "$command" --service plugin-advisory

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"final_review.plan"'* ]]
  [[ "$output" != *'"name":"workflow.start"'* ]]
  run jq -e '
    [.result.tools[].name] as $names |
    all($names[];
      . == "workspace-reader.status" or
      . == "workspace-reader.read" or
      . == "workspace-reader.list" or
      . == "workspace-reader.search" or
      . == "workspace-reader.repository" or
      . == "setup.preview" or
      . == "setup.apply" or
      . == "final_review.plan" or
      . == "final_review.filter_findings" or
      . == "final_review.advance" or
      . == "final_review.confirm_split" or
      . == "final_review.clean_status" or
      . == "final_review.out_of_scope_report" or
      . == "final_review.resume_latest" or
      . == "final_review.assess_risk"
    )
  ' <<<"$output"
  [ "$status" -eq 0 ]
}
