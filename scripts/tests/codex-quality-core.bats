#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/codex-quality-core.sh"
  TMPROOT="$(mktemp -d)"
  FAKE_CODEX_STATE="$TMPROOT/state"
  FAKE_CODEX_LOG="$TMPROOT/codex.log"
  mkdir -p "$TMPROOT/bin" "$FAKE_CODEX_STATE"

  cat >"$TMPROOT/bin/codex" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "$*" >>"$FAKE_CODEX_LOG"

case "$*" in
  "plugin marketplace list --json")
    if [ -f "$FAKE_CODEX_STATE/marketplace-added" ]; then
      jq -n --arg root "$FAKE_MARKETPLACE_ROOT" \
        '{marketplaces: [{name: "ai-plugins", root: $root}]}'
    else
      printf '{"marketplaces":[]}\n'
    fi
    ;;
  "plugin marketplace add "*" --json")
    touch "$FAKE_CODEX_STATE/marketplace-added"
    jq -n --arg root "$FAKE_MARKETPLACE_ROOT" \
      '{marketplaceName: "ai-plugins", installedRoot: $root, alreadyAdded: false}'
    ;;
  "plugin add "*"@ai-plugins --json")
    plugin="${3%@ai-plugins}"
    touch "$FAKE_CODEX_STATE/plugin-$plugin"
    jq -n --arg plugin "$plugin" \
      '{pluginId: ($plugin + "@ai-plugins"), name: $plugin, marketplaceName: "ai-plugins"}'
    ;;
  "plugin list --available --json")
    if [ "${FAKE_CODEX_MODE:-healthy}" = "missing-plugin" ]; then
      jq -n \
        --arg root "$FAKE_MARKETPLACE_ROOT" \
        '{installed: [
          {name: "engineering-standards", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
          {name: "development-discipline", marketplaceName: "ai-plugins", version: "0.11.0", installed: true, enabled: true}
        ], available: [], marketplaceRoot: $root}'
    elif [ "${FAKE_CODEX_MODE:-healthy}" = "missing-agentic" ]; then
      jq -n \
        --arg root "$FAKE_MARKETPLACE_ROOT" \
        '{installed: [
          {name: "engineering-standards", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
          {name: "development-discipline", marketplaceName: "ai-plugins", version: "0.11.0", installed: true, enabled: true},
          {name: "advisor", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true}
        ], available: [], marketplaceRoot: $root}'
    else
      jq -n \
        --arg root "$FAKE_MARKETPLACE_ROOT" \
        '{installed: [
          {name: "engineering-standards", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
          {name: "development-discipline", marketplaceName: "ai-plugins", version: "0.11.0", installed: true, enabled: true},
          {name: "advisor", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
          {name: "agentic-systems-engineering", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true}
        ], available: [], marketplaceRoot: $root}'
    fi
    ;;
  -C*" debug prompt-input "*)
    printf '%s\n' '[{"content":"engineering-standards:engineering-standards development-discipline:test-driven-development development-discipline:verification-before-completion advisor:advisor agentic-systems-engineering:agentic-systems-engineering"}]'
    ;;
  *)
    printf 'unexpected fake Codex invocation: %s\n' "$*" >&2
    exit 97
    ;;
esac
SH
  chmod +x "$TMPROOT/bin/codex"

  export FAKE_CODEX_STATE FAKE_CODEX_LOG
  export FAKE_MARKETPLACE_ROOT="$ROOT"
  export PATH="$TMPROOT/bin:$PATH"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "install adds the local marketplace and makes the quality core model-visible" {
  run "$RUNNER" install

  [ "$status" -eq 0 ]
  [[ "$output" == *"Codex quality core is installed and model-visible"* ]]
  grep -Fqx "plugin marketplace add $ROOT --json" "$FAKE_CODEX_LOG"
  grep -Fqx "plugin add engineering-standards@ai-plugins --json" "$FAKE_CODEX_LOG"
  grep -Fqx "plugin add development-discipline@ai-plugins --json" "$FAKE_CODEX_LOG"
  grep -Fqx "plugin add advisor@ai-plugins --json" "$FAKE_CODEX_LOG"
  grep -Fq "debug prompt-input" "$FAKE_CODEX_LOG"

  run grep -Fq "plugin add agentic-systems-engineering@ai-plugins" "$FAKE_CODEX_LOG"
  [ "$status" -eq 1 ]
}

@test "help does not require Codex Git or jq" {
  help_path="$TMPROOT/help-bin"
  mkdir "$help_path"
  ln -s "$(command -v bash)" "$help_path/bash"
  ln -s "$(command -v cat)" "$help_path/cat"

  run env PATH="$help_path" "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/codex-quality-core.sh install [--with-agentic]"* ]]
  [[ "$output" == *"scripts/codex-quality-core.sh check [--with-agentic] [DOWNSTREAM]"* ]]
  [[ "$output" != *"missing required command"* ]]
  [[ "$output" != *"command not found"* ]]
}

@test "agentic systems guidance is an explicit opt-in" {
  run "$RUNNER" install --with-agentic

  [ "$status" -eq 0 ]
  grep -Fqx "plugin add agentic-systems-engineering@ai-plugins --json" "$FAKE_CODEX_LOG"
}

@test "an invalid command is reported before its otherwise valid option" {
  run "$RUNNER" bogus --with-agentic

  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown command: bogus"* ]]
  [[ "$output" != *"unknown option: --with-agentic"* ]]
}

@test "check reports a missing core plugin without attempting repair" {
  touch "$FAKE_CODEX_STATE/marketplace-added"
  export FAKE_CODEX_MODE=missing-plugin

  run "$RUNNER" check

  [ "$status" -eq 1 ]
  [[ "$output" == *"missing Codex plugin: advisor@ai-plugins"* ]]
  [[ "$output" == *"rerun '$RUNNER install'"* ]]

  run grep -F "plugin add" "$FAKE_CODEX_LOG"
  [ "$status" -eq 1 ]
}

@test "agentic check preserves the opt-in flag in repair guidance" {
  touch "$FAKE_CODEX_STATE/marketplace-added"
  export FAKE_CODEX_MODE=missing-agentic

  run "$RUNNER" check --with-agentic

  [ "$status" -eq 1 ]
  [[ "$output" == *"missing Codex plugin: agentic-systems-engineering@ai-plugins"* ]]
  [[ "$output" == *"rerun '$RUNNER install --with-agentic'"* ]]
}

@test "check renders plugin context in the caller's downstream repository" {
  downstream="$TMPROOT/downstream"
  mkdir "$downstream"
  git -C "$downstream" init -q
  touch "$FAKE_CODEX_STATE/marketplace-added"

  run "$RUNNER" check "$downstream"

  [ "$status" -eq 0 ]
  grep -Fq -- "-C $downstream debug prompt-input" "$FAKE_CODEX_LOG"
  [ -z "$(git -C "$downstream" status --short)" ]
}

@test "check accepts the downstream repository before the agentic option" {
  downstream="$TMPROOT/downstream"
  mkdir "$downstream"
  git -C "$downstream" init -q
  touch "$FAKE_CODEX_STATE/marketplace-added"

  run "$RUNNER" check "$downstream" --with-agentic

  [ "$status" -eq 0 ]
  grep -Fq -- "-C $downstream debug prompt-input" "$FAKE_CODEX_LOG"
}

@test "check rejects an unknown option before querying Codex" {
  run "$RUNNER" check --with-agenti

  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown option: --with-agenti"* ]]
  [ ! -s "$FAKE_CODEX_LOG" ]
}
