#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup_file() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  export XDG_DATA_HOME="$BATS_FILE_TMPDIR/xdg-data"
  just --justfile "$ROOT/justfile" install-development-system-binaries --from-source
}

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$BATS_TEST_TMPDIR"

}

teardown() {
  if [ -n "${SIGNING_AGENT_PID:-}" ]; then
    kill "$SIGNING_AGENT_PID" 2>/dev/null || true
  fi
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

@test "Tiber reports installed-user repair when no binary is installed" {
  mkdir -p "$TMPROOT/failing-bin"
  printf '#!/bin/sh\nexit 1\n' >"$TMPROOT/failing-bin/curl"
  chmod +x "$TMPROOT/failing-bin/curl"
  run env \
    PATH="$TMPROOT/failing-bin:$PATH" \
    XDG_DATA_HOME="$TMPROOT/empty-xdg-data" \
    HOME="$TMPROOT/home" \
    "$ROOT/plugins/development-system/bin/tiber" --help

  [ "$status" -ne 0 ]
  [[ "$output" == *"development_system.binary_repair_failed"* ]]
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

  run just --justfile "$ROOT/justfile" install-development-system-binaries --from-source
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
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" --from-source

  [ "$status" -eq 0 ]
  mapfile -t directories <"$call_log"
  [ "${directories[0]}" = "$tiber_rust" ]
  [ "${directories[1]}" = "$discipline_rust" ]
}

@test "the host-local bootstrap keeps its default build cache outside the checkout" {
  local fake_bin="$TMPROOT/default-cargo-bin"
  local fake_cargo="$fake_bin/cargo"
  local target_log="$TMPROOT/default-cargo-targets"
  local data_home="$TMPROOT/default-cargo-xdg-data"

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
    'printf "%s\\n" "$CARGO_TARGET_DIR" >> "$CARGO_TARGET_LOG"' \
    'mkdir -p "$CARGO_TARGET_DIR/release"' \
    'printf "%s\\n" "#!/bin/sh" "exit 0" > "$CARGO_TARGET_DIR/release/$binary"' \
    'chmod +x "$CARGO_TARGET_DIR/release/$binary"' >"$fake_cargo"
  chmod +x "$fake_cargo"

  run env -u CARGO_TARGET_DIR \
    PATH="$fake_bin:$PATH" \
    CARGO_TARGET_LOG="$target_log" \
    XDG_DATA_HOME="$data_home" \
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" --from-source

  [ "$status" -eq 0 ]
  [ "$(wc -l <"$target_log")" -eq 2 ]
  while IFS= read -r target; do
    [[ "$target" == "$data_home/ai-plugins/development-system/build-cache/"* ]]
    [[ "$target" != "$ROOT/"* ]]
  done <"$target_log"
}

@test "concurrent host-local bootstrap runs serialize the shared build cache" {
  local fake_bin="$TMPROOT/concurrent-cargo-bin"
  local fake_cargo="$fake_bin/cargo"
  local data_home="$TMPROOT/concurrent-cargo-xdg-data"
  local sentinel="$TMPROOT/cargo-concurrency-sentinel"
  local status_dir="$TMPROOT/concurrent-status"

  mkdir -p "$fake_bin" "$status_dir"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'if [[ "$1" == "--version" ]]; then echo "cargo fake"; exit 0; fi' \
    'binary=""' \
    'while [[ $# -gt 0 ]]; do' \
    '  if [[ "$1" == "--bin" ]]; then binary="$2"; shift 2; continue; fi' \
    '  shift' \
    'done' \
    'mkdir "$CARGO_CONCURRENCY_SENTINEL" || exit 99' \
    'trap '\''rmdir "$CARGO_CONCURRENCY_SENTINEL"'\'' EXIT' \
    'sleep 0.2' \
    'mkdir -p "$CARGO_TARGET_DIR/release"' \
    'printf "%s\\n" "#!/bin/sh" "exit 0" > "$CARGO_TARGET_DIR/release/$binary"' \
    'chmod +x "$CARGO_TARGET_DIR/release/$binary"' >"$fake_cargo"
  chmod +x "$fake_cargo"

  run env -u CARGO_TARGET_DIR \
    ROOT="$ROOT" \
    PATH="$fake_bin:$PATH" \
    CARGO_CONCURRENCY_SENTINEL="$sentinel" \
    XDG_DATA_HOME="$data_home" \
    STATUS_DIR="$status_dir" \
    bash -c '
      "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" --from-source >"$STATUS_DIR/one.log" 2>&1 & one=$!
      "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" --from-source >"$STATUS_DIR/two.log" 2>&1 & two=$!
      wait "$one"; one_status=$?
      wait "$two"; two_status=$?
      printf "%s %s\n" "$one_status" "$two_status"
    '

  [ "$status" -eq 0 ]
  [ "$output" = "0 0" ]
}

@test "an interruption after publication preserves the installed binaries" {
  local fake_bin="$TMPROOT/interrupted-install-bin"
  local fake_cargo="$fake_bin/cargo"
  local fake_mv="$fake_bin/mv"
  local data_home="$TMPROOT/interrupted-install-xdg-data"
  local real_mv
  local host
  local version

  real_mv="$(command -v mv)"
  host="$(source "$ROOT/plugins/development-system/lib/installed-binary.sh"; development_system_host)"
  version="$(jq -r '.version' "$ROOT/plugins/development-system/plugin.json")"
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
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'published=false' \
    'for argument in "$@"; do [[ "$argument" == *".link."* ]] && published=true; done' \
    '"$REAL_MV" "$@"' \
    'if [[ "$published" == true ]]; then kill -TERM "$PPID"; sleep 1; fi' >"$fake_mv"
  chmod +x "$fake_cargo" "$fake_mv"

  run env -u CARGO_TARGET_DIR \
    PATH="$fake_bin:$PATH" \
    REAL_MV="$real_mv" \
    XDG_DATA_HOME="$data_home" \
    "$ROOT/plugins/development-system/scripts/install-development-system-binaries.sh" --from-source

  [ "$status" -ne 0 ]
  [ -x "$data_home/ai-plugins/development-system/$version/$host/tiber" ]
  [ -x "$data_home/ai-plugins/development-system/$version/$host/development-discipline-mcp" ]
}

@test "legacy dependency state remains ignored during migration" {
  local legacy="$ROOT/.dependencies/npmrc"

  run git -C "$ROOT" check-ignore -q "$legacy"

  [ "$status" -eq 0 ]
}

@test "the host-local bootstrap normalizes a relative Cargo target directory" {
  local fake_bin="$TMPROOT/relative-cargo-bin"
  local fake_cargo="$fake_bin/cargo"
  local relative_target="target/test-cargo-target-$BATS_TEST_NUMBER"
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
    "$ROOT/scripts/install-development-system-binaries.sh" --from-source

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

  mkdir -p "$TMPROOT/failing-bin"
  printf '#!/bin/sh\nexit 1\n' >"$TMPROOT/failing-bin/curl"
  chmod +x "$TMPROOT/failing-bin/curl"
  run env \
    PATH="$TMPROOT/failing-bin:$PATH" \
    XDG_DATA_HOME="$TMPROOT/missing-xdg-data" \
    HOME="$TMPROOT/home" \
    bash -c '"$1" --help >"$2" 2>"$3"' _ \
    "$ROOT/plugins/development-system/bin/tiber" \
    "$stdout_file" \
    "$stderr_file"

  [ "$status" -ne 0 ]
  [ ! -s "$stdout_file" ]
  [[ "$(<"$stderr_file")" == *"development_system.binary_repair_failed"* ]]
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

@test "portable Tiber MCP signs an event from an external repository through plugin data" {
  local repo="$TMPROOT/signed-repo"
  local data="$TMPROOT/plugin-data"
  local version
  version="$(jq -r .version "$ROOT/plugins/development-system/plugin.json")"
  mkdir -p "$repo" "$data/ai-plugins/development-system/$version"
  git init --bare -q "$TMPROOT/origin.git"
  git init -q "$repo"
  git -C "$repo" config user.name 'Portable MCP Test'
  git -C "$repo" config user.email 'portable@example.invalid'
  git -C "$repo" remote add origin "$TMPROOT/origin.git"
  ssh-keygen -q -t ed25519 -N '' -f "$TMPROOT/key"
  ssh-agent -a "$TMPROOT/agent.sock" -D >"$TMPROOT/agent.log" 2>&1 &
  SIGNING_AGENT_PID=$!
  for _ in $(seq 1 50); do [ -S "$TMPROOT/agent.sock" ] && break; sleep 0.1; done
  SSH_AUTH_SOCK="$TMPROOT/agent.sock" ssh-add "$TMPROOT/key" >/dev/null 2>&1
  git -C "$repo" config gpg.format ssh
  git -C "$repo" config commit.gpgsign true
  git -C "$repo" config user.signingkey "$TMPROOT/key.pub"
  (cd "$repo" && SSH_AUTH_SOCK="$TMPROOT/agent.sock" "$ROOT/plugins/development-system/bin/tiber" init)
  local before
  before="$(git -C "$repo" rev-parse refs/remotes/origin/tiber)"
  ln -s "$XDG_DATA_HOME/ai-plugins/development-system/$version/linux-x86_64" \
    "$data/ai-plugins/development-system/$version/linux-x86_64"
  printf '%s\n' "$TMPROOT/agent.sock" >"$data/signing-agent-socket"

  local request
  request="$(jq -nc --arg cwd "file://$repo" '{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:"tiber.create",arguments:{title:"Portable signed task"},_meta:{"codex/sandbox-state-meta":{sandboxCwd:$cwd}}}}')"
  run bash -c 'cd "$1"; printf "%s\n" "$2" | env -u SSH_AUTH_SOCK PLUGIN_ROOT="$1" PLUGIN_DATA="$3" "$1/bin/tiber" mcp stdio' _ \
    "$ROOT/plugins/development-system" "$request" "$data"

  [ "$status" -eq 0 ]
  [[ "$output" == *'portable-signed-task'* ]]
  local after
  after="$(git -C "$repo" rev-parse refs/remotes/origin/tiber)"
  [ "$after" != "$before" ]
  git -C "$repo" cat-file -p "$after" | grep -q '^gpgsig '
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

@test "setup leaves MCP registration to the plugin and binary paths survive reinstall" {
  local project="$TMPROOT/setup-project"
  local tiber_path
  local discipline_path
  local request

  mkdir -p "$project"
  git -C "$project" init --quiet
  printf 'ci:\n    true\n' >"$project/justfile"
  source "$ROOT/plugins/development-system/lib/installed-binary.sh"
  tiber_path="$(development_system_installed_binary_path "$ROOT/plugins/development-system" tiber)"
  discipline_path="$(development_system_installed_binary_path "$ROOT/plugins/development-system" development-discipline-mcp)"
  request="$(jq -cn --arg project_root "$project" '{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:"setup.apply",arguments:{project_root:$project_root,confirmed:true,selected_command_ids:["just-ci"]}}}')"

  run bash -c 'printf "%s\\n" "$2" | "$1" --service plugin-advisory' _ "$discipline_path" "$request"
  [ "$status" -eq 0 ]

  [ ! -e "$project/.codex/config.toml" ]

  run just --justfile "$ROOT/justfile" install-development-system-binaries --from-source
  [ "$status" -eq 0 ]
  [ -x "$discipline_path" ]
  [ -x "$tiber_path" ]
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
    ($names | index("final_review.reopen") != null) and
    ($names | index("final_review.continue_review") != null) and
    ($names | index("final_review.yield_report") != null) and
    ($names | index("final_review.evidence") != null) and
    all($names[];
      . == "workspace-reader.status" or
      . == "workspace-reader.read" or
      . == "workspace-reader.list" or
      . == "workspace-reader.search" or
      . == "workspace-reader.repository" or
      . == "setup.preview" or
      . == "setup.apply" or
      . == "development_system.codex_sandbox_setup" or
      . == "final_review.plan" or
      . == "final_review.filter_findings" or
      . == "final_review.advance" or
      . == "final_review.confirm_split" or
      . == "final_review.clean_status" or
      . == "final_review.out_of_scope_report" or
      . == "final_review.resume_latest" or
      . == "final_review.reopen" or
      . == "final_review.continue_review" or
      . == "final_review.yield_report" or
      . == "final_review.evidence" or
      . == "final_review.pending_assignments" or
      . == "final_review.assess_risk"
    )
  ' <<<"$output"
  [ "$status" -eq 0 ]
}
