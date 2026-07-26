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
version="$(jq -er '.version' "$FAKE_MARKETPLACE_ROOT/plugins/development-system/.codex-plugin/plugin.json")"

case "$*" in
  "plugin marketplace list --json")
    if [ "${FAKE_CODEX_MODE:-healthy}" = "invalid-marketplace" ]; then
      printf '{"marketplaces":{}}\n'
    elif [ -f "$FAKE_CODEX_STATE/marketplace-added" ]; then
      root="$FAKE_MARKETPLACE_ROOT"
      if [ "${FAKE_CODEX_MODE:-healthy}" = "conflicting-marketplace" ]; then
        root="$FAKE_CONFLICTING_ROOT"
      fi
      jq -n --arg root "$root" '{marketplaces:[{name:"ai-plugins",root:$root}]}'
    else
      printf '{"marketplaces":[]}\n'
    fi
    ;;
  "plugin marketplace add "*" --json")
    touch "$FAKE_CODEX_STATE/marketplace-added"
    jq -n '{marketplaceName:"ai-plugins"}'
    ;;
  "plugin add development-system@ai-plugins --json")
    touch "$FAKE_CODEX_STATE/plugin-development-system"
    jq -n '{name:"development-system",marketplaceName:"ai-plugins"}'
    ;;
  "plugin list --available --json")
    if [ "${FAKE_CODEX_MODE:-healthy}" = "invalid-plugin-schema" ]; then
      printf '{"installed":{},"available":[]}\n'
    elif [ "${FAKE_CODEX_MODE:-healthy}" = "missing-plugin" ]; then
      printf '{"installed":[],"available":[]}\n'
    else
      enabled=true
      actual="$version"
      if [ "${FAKE_CODEX_MODE:-healthy}" = "stale-plugin" ]; then
        actual="0.0.0-stale"
      elif [ "${FAKE_CODEX_MODE:-healthy}" = "disabled-plugin" ]; then
        enabled=false
      fi
      jq -n --arg version "$actual" --argjson enabled "$enabled" \
        '{installed:[{name:"development-system",marketplaceName:"ai-plugins",version:$version,installed:true,enabled:$enabled}],available:[]}'
    fi
    ;;
  -C*" debug prompt-input "*)
    if [ "${FAKE_CODEX_MODE:-healthy}" = "invalid-prompt-schema" ]; then
      printf '{}\n'
    else
      skills='- development-system:setup: Setup.
- development-system:development-workflow: Workflow.
- development-system:delivery: Delivery.
- development-system:worktrees: Worktrees.
- development-system:tasks: Tasks.
- development-system:engineering-standards: Standards.
- development-system:agentic-systems: Agentic systems.
- development-system:eval-case-reporting: Eval reporting.'
      if [ "${FAKE_CODEX_MODE:-healthy}" = "invisible-skill" ]; then
        skills='- development-system:setup: Setup.'
      fi
      jq -n --arg skills "$skills" '[{
        type:"message",
        role:"developer",
        content:[
          {type:"input_text",text:"<permissions instructions>\nRead-only.\n</permissions instructions>"},
          {type:"input_text",text:("<skills_instructions>\n## Skills\n" + $skills + "\n</skills_instructions>")},
          {type:"input_text",text:"<plugins_instructions>\nPlugin metadata.\n</plugins_instructions>"}
        ]
      }]'
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

seed_marketplace() {
  touch "$FAKE_CODEX_STATE/marketplace-added"
}

@test "install adds exactly the single plugin and verifies all router skills" {
  run "$RUNNER" install

  [ "$status" -eq 0 ]
  [[ "$output" == *"Codex Development System is installed and model-visible"* ]]
  grep -Fqx "plugin marketplace add $ROOT --json" "$FAKE_CODEX_LOG"
  [ "$(grep -c '^plugin add ' "$FAKE_CODEX_LOG")" -eq 1 ]
  grep -Fqx "plugin add development-system@ai-plugins --json" "$FAKE_CODEX_LOG"
  grep -Fq "debug prompt-input" "$FAKE_CODEX_LOG"
}

@test "help documents the single-plugin commands without requiring Codex" {
  help_path="$TMPROOT/help-bin"
  mkdir "$help_path"
  ln -s "$(command -v bash)" "$help_path/bash"
  ln -s "$(command -v cat)" "$help_path/cat"

  run env PATH="$help_path" "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/codex-quality-core.sh install"* ]]
  [[ "$output" != *"--with-agentic"* ]]
}

@test "check reports a missing plugin without repairing it" {
  seed_marketplace
  run env FAKE_CODEX_MODE=missing-plugin "$RUNNER" check

  [ "$status" -eq 1 ]
  [[ "$output" == *"missing Codex plugin: development-system@ai-plugins"* ]]
  ! grep -q '^plugin add ' "$FAKE_CODEX_LOG"
}

@test "check reports stale and disabled plugin state" {
  seed_marketplace
  run env FAKE_CODEX_MODE=stale-plugin "$RUNNER" check
  [ "$status" -eq 1 ]
  [[ "$output" == *"stale Codex plugin: development-system@ai-plugins"* ]]

  run env FAKE_CODEX_MODE=disabled-plugin "$RUNNER" check
  [ "$status" -eq 1 ]
  [[ "$output" == *"disabled Codex plugin: development-system@ai-plugins"* ]]
}

@test "install refuses a marketplace pointing at another checkout" {
  seed_marketplace
  run env FAKE_CODEX_MODE=conflicting-marketplace "$RUNNER" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"points to a different checkout"* ]]
  ! grep -q '^plugin add ' "$FAKE_CODEX_LOG"
}

@test "schema failures are distinct from missing state" {
  seed_marketplace
  run env FAKE_CODEX_MODE=invalid-marketplace "$RUNNER" check
  [ "$status" -eq 2 ]
  [[ "$output" == *"unsupported Codex marketplace schema"* ]]

  run env FAKE_CODEX_MODE=invalid-plugin-schema "$RUNNER" check
  [ "$status" -eq 2 ]
  [[ "$output" == *"unsupported Codex plugin state schema"* ]]
}

@test "check rejects incomplete or incompatible model-visible skill registries" {
  seed_marketplace
  run env FAKE_CODEX_MODE=invisible-skill "$RUNNER" check
  [ "$status" -eq 1 ]
  [[ "$output" == *"installed skill is not model-visible"* ]]

  run env FAKE_CODEX_MODE=invalid-prompt-schema "$RUNNER" check
  [ "$status" -eq 2 ]
  [[ "$output" == *"unsupported Codex prompt schema"* ]]
}

@test "check uses the caller-provided downstream repository" {
  seed_marketplace
  git -C "$TMPROOT" init -q downstream

  run "$RUNNER" check "$TMPROOT/downstream"

  [ "$status" -eq 0 ]
  grep -Fq -- "-C $TMPROOT/downstream" "$FAKE_CODEX_LOG"
}

@test "unknown options fail before querying Codex" {
  run "$RUNNER" check --with-agentic

  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown option: --with-agentic"* ]]
  [ ! -s "$FAKE_CODEX_LOG" ]
}
