#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TEST_ROOT="$(mktemp -d)"
  AUTH_HOME="$TEST_ROOT/auth"
  EVAL_HOME="$TEST_ROOT/eval-home"
  FAKE_CLAUDE="$TEST_ROOT/claude"
  CLAUDE_LOG="$TEST_ROOT/claude.log"
  mkdir -p "$AUTH_HOME"
  printf '%s\n' '{"claudeAiOauth":{"accessToken":"fixture"}}' \
    >"$AUTH_HOME/.credentials.json"
  cat >"$FAKE_CLAUDE" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$CLAUDE_LOG"
version="$(jq -r '.version' "$REPO_ROOT/plugins/development-system/.claude-plugin/plugin.json")"
install_path="$CLAUDE_CODE_PLUGIN_CACHE_DIR/cache/ai-plugins/development-system/$version"

case "$*" in
  "plugin marketplace add "*" --scope user")
    ;;
  "plugin install development-system@ai-plugins --scope user")
    mkdir -p "$install_path"
    cp -R "$REPO_ROOT/plugins/development-system/." "$install_path/"
    if [[ "${FAKE_CLAUDE_OVERWRITE_AUTH:-0}" == "1" ]]; then
      printf '%s\n' '{"stale":"installer-state"}' \
        >"$CLAUDE_CONFIG_DIR/.credentials.json"
    fi
    ;;
  "plugin list --json")
    if [[ "${FAKE_CLAUDE_PLUGIN_ERRORS:-0}" == "1" ]]; then
      errors='["Hook load failed"]'
    else
      errors='[]'
    fi
    jq -n \
      --arg installPath "$install_path" \
      --arg version "$version" \
      --argjson errors "$errors" \
      '[{
        id: "development-system@ai-plugins",
        version: $version,
        scope: "user",
        enabled: true,
        installPath: $installPath,
        errors: $errors
      }]'
    ;;
  "plugin validate "*)
    ;;
  *)
    printf 'unexpected claude invocation: %s\n' "$*" >&2
    exit 91
    ;;
esac
SH
  chmod +x "$FAKE_CLAUDE"
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

@test "Claude eval preparation installs, validates, and reports the cached plugin" {
  run env \
    CLAUDE_BIN="$FAKE_CLAUDE" \
    CLAUDE_EVAL_AUTH_HOME="$AUTH_HOME" \
    CLAUDE_LOG="$CLAUDE_LOG" \
    REPO_ROOT="$ROOT" \
    node "$ROOT/scripts/evals/prepare-claude-home.mjs" \
    "$EVAL_HOME" \
    --plugin-mode development-system

  [ "$status" -eq 0 ]
  plugin_path="$(jq -r '.pluginPath' <<<"$output")"
  [ -d "$plugin_path" ]
  [[ "$plugin_path" == "$EVAL_HOME/plugin-cache/cache/ai-plugins/development-system/"* ]]
  [ ! -e "$EVAL_HOME/config/.credentials.json" ]
  grep -Fq "plugin marketplace add $ROOT --scope user" "$CLAUDE_LOG"
  grep -Fq "plugin install development-system@ai-plugins --scope user" "$CLAUDE_LOG"
  grep -Fq "plugin validate $plugin_path" "$CLAUDE_LOG"
}

@test "Claude eval preparation never copies rotating OAuth credentials" {
  run env \
    CLAUDE_BIN="$FAKE_CLAUDE" \
    CLAUDE_EVAL_AUTH_HOME="$AUTH_HOME" \
    CLAUDE_LOG="$CLAUDE_LOG" \
    FAKE_CLAUDE_OVERWRITE_AUTH=1 \
    REPO_ROOT="$ROOT" \
    node "$ROOT/scripts/evals/prepare-claude-home.mjs" \
    "$EVAL_HOME" \
    --plugin-mode development-system

  [ "$status" -eq 0 ]
  grep -q '"stale":"installer-state"' "$EVAL_HOME/config/.credentials.json"
  grep -q '"accessToken":"fixture"' "$AUTH_HOME/.credentials.json"
}

@test "Claude eval preparation rejects installer-reported component errors" {
  run env \
    CLAUDE_BIN="$FAKE_CLAUDE" \
    CLAUDE_EVAL_AUTH_HOME="$AUTH_HOME" \
    CLAUDE_LOG="$CLAUDE_LOG" \
    FAKE_CLAUDE_PLUGIN_ERRORS=1 \
    REPO_ROOT="$ROOT" \
    node "$ROOT/scripts/evals/prepare-claude-home.mjs" \
    "$EVAL_HOME" \
    --plugin-mode development-system

  [ "$status" -eq 2 ]
  [[ "$output" == *"installed Claude plugin reported component errors"* ]]
}

@test "Claude no-plugin preparation creates an isolated home without invoking the installer" {
  run env \
    CLAUDE_BIN="$FAKE_CLAUDE" \
    CLAUDE_EVAL_AUTH_HOME="$AUTH_HOME" \
    CLAUDE_LOG="$CLAUDE_LOG" \
    REPO_ROOT="$ROOT" \
    node "$ROOT/scripts/evals/prepare-claude-home.mjs" \
    "$EVAL_HOME" \
    --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  [ "$(jq -r '.pluginPath' <<<"$output")" = "null" ]
  [ ! -e "$CLAUDE_LOG" ]
  [ ! -e "$EVAL_HOME/config/.credentials.json" ]
}
