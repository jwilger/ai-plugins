#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  BOUNDARY="$ROOT/scripts/evals/behavior-provider-boundary.sh"
  FIXTURE="$(mktemp -d)"
  HOME_DIR="$FIXTURE/home"
  WORKSPACE="$FIXTURE/workspace"
  PLUGIN="$HOME_DIR/plugin-snapshot"
  PROVIDER="$FIXTURE/claude"
  HOST_CANARY="$FIXTURE/host-canary-$(date +%s%N)"
  BWRAP_BIN="$(realpath "$(command -v bwrap)")"
  BWRAP_STORE_ROOT="${BWRAP_BIN#/nix/store/}"
  BWRAP_STORE_ROOT="/nix/store/${BWRAP_STORE_ROOT%%/*}"
  mkdir -p "$HOME_DIR" "$WORKSPACE" "$PLUGIN"
  printf 'condition-owned\n' >"$HOME_DIR/sentinel"
  printf 'ai-plugins Claude eval home\n' >"$HOME_DIR/.ai-plugins-claude-eval-home"
  printf 'ai-plugins Codex eval home\n' >"$HOME_DIR/.ai-plugins-eval-home"
  printf 'host-only\n' >"$HOST_CANARY"
  printf 'sanitized plugin\n' >"$PLUGIN/SKILL.md"
  cat >"$PROVIDER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[ "$PWD" = /workspace ]
[ "$HOME" = /runtime/home ]
[ -f /runtime/home/sentinel ]
[ -f /runtime/plugin/SKILL.md ]
[ -r "$SSL_CERT_FILE" ]
[ "${ANTHROPIC_API_KEY:-}" = fixture-key ]
[ -x /bin/bash ]
/bin/bash -c '[[ -n ${BASH_VERSION:-} ]]'
[ ! -e "$1" ]
[ ! -e "$2" ]
printf 'runtime-only\n' > /runtime/home/runtime-write
printf 'runtime-only\n' > /workspace/runtime-write
SH
  chmod +x "$PROVIDER"
}

teardown() {
  rm -rf "$FIXTURE"
}

@test "provider boundary exposes only condition-owned workspace home and plugin snapshot" {
  run env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=development-system \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_PLUGIN_SNAPSHOT="$PLUGIN" \
    EVAL_PROVIDER_SESSION_START_EVIDENCE="$HOME_DIR/session-start-marker" \
    EVAL_PROVIDER_REAL_BIN="$PROVIDER" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    ANTHROPIC_API_KEY=fixture-key \
    "$BOUNDARY" "$HOST_CANARY" "$BWRAP_STORE_ROOT"

  [ "$status" -eq 0 ]
  [ ! -e "$HOME_DIR/runtime-write" ]
[ ! -e "$WORKSPACE/runtime-write" ]
}

@test "provider boundary preserves SessionStart evidence without sharing the cloned home" {
  cat >>"$PROVIDER" <<'SH'
: >"$DEVELOPMENT_SYSTEM_EVAL_SESSION_START_MARKER"
SH
  evidence="$HOME_DIR/session-start-marker"

  run env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=development-system \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_PLUGIN_SNAPSHOT="$PLUGIN" \
    EVAL_PROVIDER_SESSION_START_EVIDENCE="$evidence" \
    EVAL_PROVIDER_REAL_BIN="$PROVIDER" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    ANTHROPIC_API_KEY=fixture-key \
    "$BOUNDARY" "$HOST_CANARY" "$BWRAP_STORE_ROOT"

  [ "$status" -eq 0 ]
  [ -f "$evidence" ]
  [ ! -e "$HOME_DIR/runtime-write" ]
}

@test "no-plugin boundary fails closed if a plugin snapshot is supplied" {
  run env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_PLUGIN_SNAPSHOT="$PLUGIN" \
    EVAL_PROVIDER_REAL_BIN="$PROVIDER" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY"

  [ "$status" -eq 64 ]
  [[ "$output" == *"EVAL_PROVIDER_BOUNDARY_ERROR:no-plugin-condition-has-plugin-snapshot"* ]]
}

@test "Codex native runtime starts through the same no-plugin boundary" {
  printf '' >"$HOME_DIR/config.toml"

  run env \
    EVAL_PROVIDER_HARNESS=codex \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_NODE_BIN="$(realpath "$(command -v node)")" \
    EVAL_PROVIDER_CODEX_RUNTIME="$(realpath "$ROOT/tooling/evals/node_modules/@openai")" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY" exec --cd "$WORKSPACE" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Run Codex non-interactively"* ]]
}

@test "Codex boundary forwards only the child SDK credential into the condition" {
  printf '' >"$HOME_DIR/config.toml"
  fake_runtime="$FIXTURE/openai-runtime"
  mkdir -p "$fake_runtime/codex/bin"
  cat >"$fake_runtime/codex/bin/codex.js" <<'JS'
if (process.env.CODEX_API_KEY !== 'fixture-key') process.exit(71);
if (process.env.ANTHROPIC_API_KEY) process.exit(72);
process.stdout.write('credential isolated\n');
JS

  run env \
    EVAL_PROVIDER_HARNESS=codex \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_NODE_BIN="$(realpath "$(command -v node)")" \
    EVAL_PROVIDER_CODEX_RUNTIME="$(realpath "$fake_runtime")" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    CODEX_API_KEY=fixture-key \
    ANTHROPIC_API_KEY=must-not-cross \
    "$BOUNDARY" exec --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"credential isolated"* ]]
}

@test "provider boundary rejects repository and plugin source trees as condition inputs" {
  run env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$ROOT" \
    EVAL_PROVIDER_REAL_BIN="$PROVIDER" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY"

  [ "$status" -eq 64 ]
  [[ "$output" == *"EVAL_PROVIDER_BOUNDARY_ERROR:workspace-overlaps-repository-source"* ]]

  run env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=development-system \
    EVAL_PROVIDER_HOME="$ROOT/plugins/development-system" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_PLUGIN_SNAPSHOT="$ROOT/plugins/development-system" \
    EVAL_PROVIDER_REAL_BIN="$PROVIDER" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY"

  [ "$status" -eq 64 ]
  [[ "$output" == *"EVAL_PROVIDER_BOUNDARY_ERROR:provider-home-overlaps-plugin-source"* ]]
}

@test "provider boundary requires a prepared sanitized condition home" {
  unprepared_home="$FIXTURE/unprepared-home"
  mkdir "$unprepared_home"

  run env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$unprepared_home" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_REAL_BIN="$PROVIDER" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY"

  [ "$status" -eq 64 ]
  [[ "$output" == *"EVAL_PROVIDER_BOUNDARY_ERROR:provider-home-not-sanitized"* ]]
}

@test "concurrent calls clone condition home and workspace per invocation" {
  concurrent_provider="$FIXTURE/concurrent-claude"
  cat >"$concurrent_provider" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
invocation_id=$1
printf '%s\n' "$invocation_id" > /runtime/home/concurrent-state
printf '%s\n' "$invocation_id" > /workspace/concurrent-state
sleep 0.5
[[ "$(</runtime/home/concurrent-state)" == "$invocation_id" ]]
[[ "$(</workspace/concurrent-state)" == "$invocation_id" ]]
SH
  chmod +x "$concurrent_provider"

  env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_REAL_BIN="$concurrent_provider" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY" first >"$FIXTURE/first.out" 2>&1 &
  first_pid=$!
  env \
    EVAL_PROVIDER_HARNESS=claude \
    EVAL_PLUGIN_MODE=no-plugins \
    EVAL_PROVIDER_HOME="$HOME_DIR" \
    EVAL_PROVIDER_WORKSPACE="$WORKSPACE" \
    EVAL_PROVIDER_REAL_BIN="$concurrent_provider" \
    EVAL_PROVIDER_BWRAP_BIN="$BWRAP_BIN" \
    "$BOUNDARY" second >"$FIXTURE/second.out" 2>&1 &
  second_pid=$!

  first_status=0
  second_status=0
  wait "$first_pid" || first_status=$?
  wait "$second_pid" || second_status=$?

  [ "$first_status" -eq 0 ]
  [ "$second_status" -eq 0 ]
  [ ! -e "$HOME_DIR/concurrent-state" ]
  [ ! -e "$WORKSPACE/concurrent-state" ]
}
