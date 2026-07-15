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
    jq -n \
      --arg root "$FAKE_MARKETPLACE_ROOT" \
      '{installed: [
        {name: "engineering-standards", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
        {name: "development-discipline", marketplaceName: "ai-plugins", version: "0.11.0", installed: true, enabled: true},
        {name: "advisor", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true},
        {name: "agentic-systems-engineering", marketplaceName: "ai-plugins", version: "0.2.0", installed: true, enabled: true}
      ], available: [], marketplaceRoot: $root}'
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
  [[ "$output" == *"Usage: scripts/codex-quality-core.sh install"* ]]
  [[ "$output" != *"missing required command"* ]]
  [[ "$output" != *"command not found"* ]]
}
