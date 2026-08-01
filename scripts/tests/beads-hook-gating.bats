#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TEST_ROOT="$(mktemp -d)"
  HOOK_PLUGIN_ROOT="$TEST_ROOT/development-system"
  BEADS_HOOK_TEST_LOG="$TEST_ROOT/beads-hook.log"
  export REPO_ROOT TEST_ROOT HOOK_PLUGIN_ROOT BEADS_HOOK_TEST_LOG

  mkdir -p "$HOOK_PLUGIN_ROOT/bin"
  cp "$REPO_ROOT/plugins/development-system/bin/run-beads-hook" \
    "$HOOK_PLUGIN_ROOT/bin/run-beads-hook"
  cat >"$HOOK_PLUGIN_ROOT/bin/bd" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$BEADS_HOOK_TEST_LOG"
if [[ -n "${BEADS_HOOK_TEST_FAILURE:-}" ]]; then
  printf '%s\n' "$BEADS_HOOK_TEST_FAILURE" >&2
  exit 86
fi
if [[ -n "${BEADS_HOOK_TEST_CWD:-}" ]]; then
  printf '%s\n' "$PWD" >"$BEADS_HOOK_TEST_CWD"
fi
if [[ -n "${BEADS_HOOK_TEST_STDIN:-}" ]]; then
  cat >"$BEADS_HOOK_TEST_STDIN"
fi
if [[ -n "${BEADS_HOOK_TEST_OUTPUT:-}" ]]; then
  printf '%s\n' "$BEADS_HOOK_TEST_OUTPUT"
fi
SH
  chmod +x "$HOOK_PLUGIN_ROOT/bin/run-beads-hook" "$HOOK_PLUGIN_ROOT/bin/bd"
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

manifest_hook_command() {
  local manifest=$1 event=$2
  jq -er --arg event "$event" '
    [.hooks[$event][]?.hooks[]?.command | select(contains("/bin/run-beads-hook"))] |
    if length == 1 then .[0] else error("expected one feature-gated Beads hook") end
  ' "$manifest"
}

initialize_git_project() {
  local project=$1
  mkdir -p "$project"
  git -C "$project" init --initial-branch=main -q
}

run_hook_command() {
  local root_variable=$1 project=$2 command=$3
  run env \
    "$root_variable=$HOOK_PLUGIN_ROOT" \
    BEADS_HOOK_TEST_LOG="$BEADS_HOOK_TEST_LOG" \
    bash -c 'cd "$1"; exec bash -c "$2"' hook-manifest-test "$project" "$command"
}

run_claude_hook() {
  local project=$1
  local command
  command="$(manifest_hook_command "$REPO_ROOT/plugins/development-system/hooks/hooks.json" SessionStart)"
  run_hook_command CLAUDE_PLUGIN_ROOT "$project" "$command"
}

run_codex_hook() {
  local project=$1 event=$2
  local command
  command="$(manifest_hook_command "$REPO_ROOT/plugins/development-system/hooks/codex.json" "$event")"
  run_hook_command PLUGIN_ROOT "$project" "$command"
}

@test "Claude Beads hook no-ops until Beads is explicitly enabled" {
  local non_git_project="$TEST_ROOT/non-git-project"
  local project="$TEST_ROOT/claude-project"
  local working_directory="$project/nested"
  mkdir -p "$non_git_project"
  printf '%s\n' '[features]' 'beads = true' >"$non_git_project/.development-system.toml"

  run_claude_hook "$non_git_project"
  [ "$status" -eq 0 ]
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  initialize_git_project "$project"
  mkdir -p "$working_directory"

  run_claude_hook "$working_directory"
  [ "$status" -eq 0 ]
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  printf '%s\n' '[features]' 'beads = false' >"$project/.development-system.toml"
  run_claude_hook "$working_directory"
  [ "$status" -eq 0 ]
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  printf '%s\n' '[features]' 'worktrees = true' >"$project/.development-system.toml"
  run_claude_hook "$working_directory"
  [ "$status" -eq 0 ]
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  printf '%s\n' 'note = """' '[features]' 'beads = true' '"""' \
    >"$project/.development-system.toml"
  run_claude_hook "$working_directory"
  [ "$status" -eq 0 ]
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  printf '%s\n' '[features]' 'beads = true' >"$project/.development-system.toml"
  run_claude_hook "$working_directory"
  [ "$status" -eq 0 ]
  [ "$(cat "$BEADS_HOOK_TEST_LOG")" = 'prime --hook-json' ]
}

@test "Codex Beads lifecycle hooks no-op until Beads is explicitly enabled" {
  local project="$TEST_ROOT/codex-project"
  local working_directory="$project/nested"
  local event
  initialize_git_project "$project"
  mkdir -p "$working_directory"

  for event in SessionStart PreCompact PostCompact UserPromptSubmit; do
    run_codex_hook "$working_directory" "$event"
    [ "$status" -eq 0 ]
  done
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  printf '%s\n' '[features]' 'beads = false' >"$project/.development-system.toml"
  for event in SessionStart PreCompact PostCompact UserPromptSubmit; do
    run_codex_hook "$working_directory" "$event"
    [ "$status" -eq 0 ]
  done
  [ ! -s "$BEADS_HOOK_TEST_LOG" ]

  printf '%s\n' '[features]' 'beads = true' >"$project/.development-system.toml"
  for event in SessionStart PreCompact PostCompact UserPromptSubmit; do
    run_codex_hook "$working_directory" "$event"
    [ "$status" -eq 0 ]
  done
  [ "$(cat "$BEADS_HOOK_TEST_LOG")" = $'codex-hook SessionStart\ncodex-hook PreCompact\ncodex-hook PostCompact\ncodex-hook UserPromptSubmit' ]
}

@test "enabled Beads hook runs the verified launcher from the Git root without changing argv, stdin, or output" {
  local project="$TEST_ROOT/project"
  local working_directory="$project/nested"
  local stdin_capture="$TEST_ROOT/stdin"
  local cwd_capture="$TEST_ROOT/cwd"
  initialize_git_project "$project"
  mkdir -p "$working_directory"
  printf '%s\n' '[features]' 'beads = true' >"$project/.development-system.toml"

  run env \
    BEADS_HOOK_TEST_LOG="$BEADS_HOOK_TEST_LOG" \
    BEADS_HOOK_TEST_CWD="$cwd_capture" \
    BEADS_HOOK_TEST_STDIN="$stdin_capture" \
    BEADS_HOOK_TEST_OUTPUT='verified-launcher-output' \
    bash -c 'cd "$1"; printf "%s\\n" "hook stdin" | "$2" --project "$PWD" -- codex-hook UserPromptSubmit --fixture-argument' \
    hook-preservation-test "$working_directory" "$HOOK_PLUGIN_ROOT/bin/run-beads-hook"

  [ "$status" -eq 0 ]
  [ "$output" = 'verified-launcher-output' ]
  [ "$(cat "$BEADS_HOOK_TEST_LOG")" = 'codex-hook UserPromptSubmit --fixture-argument' ]
  [ "$(cat "$stdin_capture")" = 'hook stdin' ]
  [ "$(cat "$cwd_capture")" = "$project" ]
}

@test "enabled Beads hook surfaces an unavailable verified launcher" {
  local project="$TEST_ROOT/project"
  initialize_git_project "$project"
  printf '%s\n' '[features]' 'beads = true' >"$project/.development-system.toml"

  run env \
    BEADS_HOOK_TEST_LOG="$BEADS_HOOK_TEST_LOG" \
    BEADS_HOOK_TEST_FAILURE='development_system.tool_unavailable tool=bd minimum=1.1.2 retry="development-system setup --enable beads"' \
    bash -c 'cd "$1"; exec "$2" --project "$PWD" -- prime --hook-json' \
    hook-failure-test "$project" "$HOOK_PLUGIN_ROOT/bin/run-beads-hook"

  [ "$status" -eq 86 ]
  [[ "$output" == *'development_system.tool_unavailable tool=bd minimum=1.1.2'* ]]
  [[ "$output" == *'development-system setup --enable beads'* ]]
  [ "$(cat "$BEADS_HOOK_TEST_LOG")" = 'prime --hook-json' ]
}

@test "installed hook manifests route each Beads lifecycle event through one feature gate" {
  run jq -e '
    ([.hooks.SessionStart[].hooks[].command | select(contains("/bin/run-beads-hook"))] | length == 1) and
    ([.hooks.SessionStart[].hooks[].command | select(contains("/bin/run-beads-hook"))] |
      .[0] == "\"${CLAUDE_PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- prime --hook-json") and
    ([.hooks[][]?.hooks[]?.command | select(contains("/bin/bd\""))] | length == 0)
  ' "$REPO_ROOT/plugins/development-system/hooks/hooks.json"
  [ "$status" -eq 0 ]

  run jq -e '
    def command_for($event; $command):
      [.hooks[$event][]?.hooks[]?.command | select(contains("/bin/run-beads-hook"))] == [$command];
    command_for("SessionStart"; "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook SessionStart") and
    command_for("PreCompact"; "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook PreCompact") and
    command_for("PostCompact"; "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook PostCompact") and
    command_for("UserPromptSubmit"; "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook UserPromptSubmit") and
    ([.hooks[][]?.hooks[]?.command | select(contains("/bin/bd\""))] | length == 0)
  ' "$REPO_ROOT/plugins/development-system/hooks/codex.json"
  [ "$status" -eq 0 ]
}
