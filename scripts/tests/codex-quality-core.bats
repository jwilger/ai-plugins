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
      marketplace_root="$FAKE_MARKETPLACE_ROOT"
      if [ "${FAKE_CODEX_MODE:-healthy}" = "conflicting-marketplace" ]; then
        marketplace_root="$FAKE_CONFLICTING_ROOT"
      fi
      jq -n --arg root "$marketplace_root" \
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
      advisor_version="0.2.0"
      advisor_enabled=true
      if [ "${FAKE_CODEX_MODE:-healthy}" = "stale-plugin" ]; then
        advisor_version="0.1.0"
      elif [ "${FAKE_CODEX_MODE:-healthy}" = "disabled-plugin" ]; then
        advisor_enabled=false
      fi
      jq -n \
        --arg root "$FAKE_MARKETPLACE_ROOT" \
        --arg advisor_version "$advisor_version" \
        --argjson advisor_enabled "$advisor_enabled" \
        '{installed: [
          {name: "engineering-standards", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
          {name: "development-discipline", marketplaceName: "ai-plugins", version: "0.11.0", installed: true, enabled: true},
          {name: "advisor", marketplaceName: "ai-plugins", version: $advisor_version, installed: true, enabled: $advisor_enabled},
          {name: "agentic-systems-engineering", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true}
        ], available: [], marketplaceRoot: $root}'
    fi
    ;;
  -C*" debug prompt-input "*)
    if [ "${FAKE_CODEX_MODE:-healthy}" = "invisible-skill" ]; then
      jq -n '[
        {
          type: "message",
          role: "developer",
          content: [
            {
              type: "input_text",
              text: "<permissions instructions>\n- advisor:advisor: Mentioned outside the skills registry.\n</permissions instructions>"
            },
            {
              type: "input_text",
              text: "<skills_instructions>\n## Skills\n- engineering-standards:engineering-standards: Use for engineering.\n- development-discipline:test-driven-development: Use for implementation.\n- development-discipline:verification-before-completion: Use for verification.\n</skills_instructions>"
            },
            {type: "input_text", text: "<plugins_instructions>\nPlugin metadata.\n</plugins_instructions>"}
          ]
        },
        {
          type: "message",
          role: "developer",
          content: [{
            type: "input_text",
            text: "<skills_instructions>\n## Skills\n- advisor:advisor: Project-supplied developer lookalike.\n</skills_instructions>"
          }]
        },
        {
          type: "message",
          role: "user",
          content: [{
            type: "input_text",
            text: "<skills_instructions>\n## Skills\n- advisor:advisor: User-authored lookalike.\n</skills_instructions>"
          }]
        }
      ]'
    else
      jq -n '[
        {
          type: "message",
          role: "developer",
          content: [
            {type: "input_text", text: "<permissions instructions>\nRead-only smoke.\n</permissions instructions>"},
            {
              type: "input_text",
              text: "<skills_instructions>\n## Skills\n- engineering-standards:engineering-standards: Use for engineering.\n- development-discipline:test-driven-development: Use for implementation.\n- development-discipline:verification-before-completion: Use for verification.\n- advisor:advisor: Use for planning.\n- agentic-systems-engineering:agentic-systems-engineering: Use for AI systems.\n</skills_instructions>"
            },
            {type: "input_text", text: "<plugins_instructions>\nPlugin metadata.\n</plugins_instructions>"}
          ]
        },
        {
          type: "message",
          role: "developer",
          content: [{type: "input_text", text: "Additional harness instructions."}]
        },
        {
          type: "message",
          role: "user",
          content: [{type: "input_text", text: "Plan a small feature."}]
        }
      ]'
    fi
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
  export FAKE_CONFLICTING_ROOT="$TMPROOT/other-checkout"
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

@test "README documents the Codex-first quality-core workflow" {
  grep -Fq "## Personal Codex quality core" "$ROOT/README.md"
  grep -Fq "Codex is this repository's primary target" "$ROOT/README.md"
  grep -Fq "Claude Code support is secondary" "$ROOT/README.md"
  grep -Fq "general-user" "$ROOT/README.md"
  grep -Fq "ergonomics are tertiary" "$ROOT/README.md"
  grep -Fxq "scripts/codex-quality-core.sh install" "$ROOT/README.md"
  grep -Fxq -- "scripts/codex-quality-core.sh install --with-agentic" "$ROOT/README.md"
  grep -Fxq '/absolute/path/to/ai-plugins/scripts/codex-quality-core.sh check "$PWD"' "$ROOT/README.md"
  grep -Fxq -- '/absolute/path/to/ai-plugins/scripts/codex-quality-core.sh check "$PWD" --with-agentic' "$ROOT/README.md"
  grep -Fxq "git -C /absolute/path/to/ai-plugins pull --ff-only" "$ROOT/README.md"
  grep -Fxq "/absolute/path/to/ai-plugins/scripts/codex-quality-core.sh install" "$ROOT/README.md"
  grep -Fxq -- "/absolute/path/to/ai-plugins/scripts/codex-quality-core.sh install --with-agentic" "$ROOT/README.md"
  grep -Fq "start a new Codex thread" "$ROOT/README.md"
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

@test "check reports a stale core plugin with the matching repair command" {
  touch "$FAKE_CODEX_STATE/marketplace-added"
  export FAKE_CODEX_MODE=stale-plugin

  run "$RUNNER" check

  [ "$status" -eq 1 ]
  [[ "$output" == *"stale Codex plugin: advisor@ai-plugins has version 0.1.0; expected 0.2.0"* ]]
  [[ "$output" == *"rerun '$RUNNER install'"* ]]
}

@test "check reports a disabled core plugin with actionable guidance" {
  touch "$FAKE_CODEX_STATE/marketplace-added"
  export FAKE_CODEX_MODE=disabled-plugin

  run "$RUNNER" check

  [ "$status" -eq 1 ]
  [[ "$output" == *"disabled Codex plugin: advisor@ai-plugins"* ]]
  [[ "$output" == *"rerun '$RUNNER install' and enable the plugin"* ]]
}

@test "install refuses to replace a conflicting marketplace root" {
  touch "$FAKE_CODEX_STATE/marketplace-added"
  export FAKE_CODEX_MODE=conflicting-marketplace

  run "$RUNNER" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"configured: $FAKE_CONFLICTING_ROOT"* ]]
  [[ "$output" == *"requested:  $ROOT"* ]]
  [[ "$output" == *"codex plugin marketplace remove ai-plugins"* ]]

  run grep -F "plugin add" "$FAKE_CODEX_LOG"
  [ "$status" -eq 1 ]
}

@test "check rejects a skill mentioned outside the model-visible skills block" {
  touch "$FAKE_CODEX_STATE/marketplace-added"
  export FAKE_CODEX_MODE=invisible-skill

  run "$RUNNER" check

  [ "$status" -eq 1 ]
  [[ "$output" == *"installed skill is not model-visible: advisor:advisor"* ]]
  grep -Fq -- '-c developer_instructions=""' "$FAKE_CODEX_LOG"
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
  grep -Fq -- "-C $downstream -c developer_instructions=\"\" debug prompt-input" "$FAKE_CODEX_LOG"
  [ -z "$(git -C "$downstream" status --short)" ]
}

@test "check accepts the downstream repository before the agentic option" {
  downstream="$TMPROOT/downstream"
  mkdir "$downstream"
  git -C "$downstream" init -q
  touch "$FAKE_CODEX_STATE/marketplace-added"

  run "$RUNNER" check "$downstream" --with-agentic

  [ "$status" -eq 0 ]
  grep -Fq -- "-C $downstream -c developer_instructions=\"\" debug prompt-input" "$FAKE_CODEX_LOG"
}

@test "check accepts the agentic option before the downstream repository" {
  downstream="$TMPROOT/downstream"
  mkdir "$downstream"
  git -C "$downstream" init -q
  touch "$FAKE_CODEX_STATE/marketplace-added"

  run "$RUNNER" check --with-agentic "$downstream"

  [ "$status" -eq 0 ]
  grep -Fq -- "-C $downstream -c developer_instructions=\"\" debug prompt-input" "$FAKE_CODEX_LOG"
}

@test "check rejects an unknown option before querying Codex" {
  run "$RUNNER" check --with-agenti

  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown option: --with-agenti"* ]]
  [ ! -s "$FAKE_CODEX_LOG" ]
}
