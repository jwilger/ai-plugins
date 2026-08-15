#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup_file() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  export XDG_DATA_HOME="$BATS_FILE_TMPDIR/xdg-data"
  just --justfile "$ROOT/justfile" install-development-system-binaries
}

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
      XDG_DATA_HOME="$XDG_DATA_HOME" \
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
      XDG_DATA_HOME="$XDG_DATA_HOME" \
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
    "XDG_DATA_HOME=$XDG_DATA_HOME"
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

@test "Tiber requires the versioned host-local installation when no binary is installed" {
  run env \
    XDG_DATA_HOME="$TMPROOT/empty-xdg-data" \
    HOME="$TMPROOT/home" \
    "$ROOT/plugins/development-system/bin/tiber" --help

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.binary_missing"* ]]
  [[ "$output" == *"just install-development-system-binaries"* ]]
}

@test "the host-local bootstrap is safe to rerun" {
  local tiber_path
  local discipline_path
  local host_link
  local first_target
  local second_target
  source "$ROOT/plugins/development-system/lib/installed-binary.sh"
  tiber_path="$(development_system_installed_binary_path "$ROOT/plugins/development-system" tiber)"
  discipline_path="$(development_system_installed_binary_path "$ROOT/plugins/development-system" development-discipline-mcp)"
  [ -x "$tiber_path" ]
  [ -x "$discipline_path" ]
  host_link="$(dirname "$tiber_path")"
  first_target="$(readlink "$host_link")"
  [[ -n "$first_target" ]]

  run just --justfile "$ROOT/justfile" install-development-system-binaries
  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.binaries_installed"* ]]
  [ -x "$tiber_path" ]
  [ -x "$discipline_path" ]
  second_target="$(readlink "$host_link")"
  [[ -n "$second_target" ]]
  [ "$first_target" != "$second_target" ]
  [ ! -e "$(dirname "$host_link")/$first_target" ]
}

@test "the host-local bootstrap builds from each pinned component directory" {
  local fake_bin="$TMPROOT/fake-cargo-bin"
  local fake_target="$TMPROOT/fake-cargo-target"
  local call_log="$TMPROOT/cargo-working-directories"
  local fake_cargo="$fake_bin/cargo"
  local tiber_rust="$ROOT/plugins/development-system/components/tiber/rust"
  local discipline_rust="$ROOT/plugins/development-system/components/development-discipline/rust"
  local -a directories

  mkdir -p "$fake_bin"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'if [[ "$1" == "--version" ]]; then echo "cargo fake"; exit 0; fi' \
    'binary=""' \
    'while [[ $# -gt 0 ]]; do' \
    '  if [[ "$1" == "--bin" ]]; then binary="$2"; shift 2; continue; fi' \
    '  shift' \
    'done' \
    'printf "%s\\n" "$PWD" >> "$CARGO_CALL_LOG"' \
    'mkdir -p "$CARGO_TARGET_DIR/release"' \
    'printf "%s\\n" "#!/bin/sh" "exit 0" > "$CARGO_TARGET_DIR/release/$binary"' \
    'chmod +x "$CARGO_TARGET_DIR/release/$binary"' >"$fake_cargo"
  chmod +x "$fake_cargo"

  run env \
    PATH="$fake_bin:$PATH" \
    CARGO_TARGET_DIR="$fake_target" \
    CARGO_CALL_LOG="$call_log" \
    XDG_DATA_HOME="$TMPROOT/fake-cargo-xdg-data" \
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh"

  [ "$status" -eq 0 ]
  mapfile -t directories <"$call_log"
  [ "${directories[0]}" = "$tiber_rust" ]
  [ "${directories[1]}" = "$discipline_rust" ]
}

@test "the host-local bootstrap normalizes a relative Cargo target directory" {
  local fake_bin="$TMPROOT/relative-cargo-bin"
  local fake_cargo="$fake_bin/cargo"
  local relative_target=".dependencies/test-cargo-target-$BATS_TEST_NUMBER"
  local expected_target="$ROOT/$relative_target"

  mkdir -p "$fake_bin"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'if [[ "$1" == "--version" ]]; then echo "cargo fake"; exit 0; fi' \
    'binary=""' \
    'while [[ $# -gt 0 ]]; do' \
    '  if [[ "$1" == "--bin" ]]; then binary="$2"; shift 2; continue; fi' \
    '  shift' \
    'done' \
    'mkdir -p "$CARGO_TARGET_DIR/release"' \
    'printf "%s\\n" "#!/bin/sh" "exit 0" > "$CARGO_TARGET_DIR/release/$binary"' \
    'chmod +x "$CARGO_TARGET_DIR/release/$binary"' >"$fake_cargo"
  chmod +x "$fake_cargo"

  run env \
    PATH="$fake_bin:$PATH" \
    CARGO_TARGET_DIR="$relative_target" \
    XDG_DATA_HOME="$TMPROOT/relative-cargo-xdg-data" \
    "$ROOT/scripts/install-development-system-binaries.sh"

  rm -rf -- "$expected_target"
  [ "$status" -eq 0 ]
}

@test "host-local binaries require an absolute XDG data location" {
  local helper="$ROOT/plugins/development-system/lib/installed-binary.sh"

  run env \
    XDG_DATA_HOME="relative-data" \
    HOME="$TMPROOT/home" \
    bash -c 'source "$1"; development_system_data_home' _ "$helper"

  [ "$status" -eq 0 ]
  [ "$output" = "$TMPROOT/home/.local/share" ]

  run env \
    XDG_DATA_HOME="relative-data" \
    HOME="relative-home" \
    bash -c 'source "$1"; development_system_data_home' _ "$helper"

  [ "$status" -ne 0 ]
}

@test "missing host-local binaries write remediation to stderr" {
  local stdout_file="$TMPROOT/launcher.stdout"
  local stderr_file="$TMPROOT/launcher.stderr"

  run env \
    XDG_DATA_HOME="$TMPROOT/missing-xdg-data" \
    HOME="$TMPROOT/home" \
    bash -c '"$1" --help >"$2" 2>"$3"' _ \
    "$ROOT/plugins/development-system/bin/tiber" \
    "$stdout_file" \
    "$stderr_file"

  [ "$status" -ne 0 ]
  [ ! -s "$stdout_file" ]
  [[ "$(<"$stderr_file")" == *"development_system.binary_missing"* ]]
}

@test "installed launchers start both MCPs without invoking Cargo" {
  local fake_bin="$TMPROOT/fake-bin"
  mkdir -p "$fake_bin"
  printf '%s\n' '#!/bin/sh' 'echo cargo-must-not-run >&2' 'exit 99' >"$fake_bin/cargo"
  chmod +x "$fake_bin/cargo"

  run env PATH="$fake_bin:$PATH" bash -c '
    printf "%s\\n" "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"bats\",\"version\":\"0.0.0\"}}}" |
      "$1" --service plugin-advisory
  ' _ "$ROOT/plugins/development-system/bin/development-discipline-mcp"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]
  [[ "$output" != *"cargo-must-not-run"* ]]

  run env PATH="$fake_bin:$PATH" \
    "$ROOT/plugins/development-system/bin/tiber" --help
  [ "$status" -eq 0 ]
  [[ "$output" == *"Repository-local task board"* ]]
  [[ "$output" != *"cargo-must-not-run"* ]]
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

@test "installed direct binaries start both MCPs from absolute paths" {
  local tiber_path
  local discipline_path

  source "$ROOT/plugins/development-system/lib/installed-binary.sh"
  tiber_path="$(development_system_installed_binary_path "$ROOT/plugins/development-system" tiber)"
  discipline_path="$(development_system_installed_binary_path "$ROOT/plugins/development-system" development-discipline-mcp)"

  run initialize_server "$discipline_path" --service plugin-advisory
  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"development-discipline"'* ]]

  run initialize_server "$tiber_path" mcp stdio
  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"tiber"'* ]]
}

@test "installed direct development-discipline binary exposes advisory coordination" {
  source "$ROOT/plugins/development-system/lib/installed-binary.sh"
  local command
  command="$(development_system_installed_binary_path "$ROOT/plugins/development-system" development-discipline-mcp)"

  run list_server_tools "$command" --service plugin-advisory

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
      . == "final_review.pending_assignments" or
      . == "final_review.assess_risk"
    )
  ' <<<"$output"
  [ "$status" -eq 0 ]
}
