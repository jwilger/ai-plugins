#!/usr/bin/env bats

setup() {
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  export REPO_ROOT
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

@test "setup refuses a linked worktree and identifies the primary checkout" {
  git -C "$TEST_ROOT" init --initial-branch=main primary
  git -C "$TEST_ROOT/primary" config user.email test@example.com
  git -C "$TEST_ROOT/primary" config user.name "Test User"
  touch "$TEST_ROOT/primary/README.md"
  git -C "$TEST_ROOT/primary" add README.md
  git -C "$TEST_ROOT/primary" commit -m "test: initialize fixture"
  git -C "$TEST_ROOT/primary" worktree add -b feature "$TEST_ROOT/linked"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/linked" \
    --preset personal-trunk \
    --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.setup_requires_primary_checkout"* ]]
  [[ "$output" == *"$TEST_ROOT/primary"* ]]
  [ ! -e "$TEST_ROOT/linked/.development-system.toml" ]
}

@test "setup refuses a bare repository because it is not a checkout" {
  git -C "$TEST_ROOT" init --bare bare.git

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/bare.git" \
    --preset personal-trunk \
    --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.setup_requires_checkout"* ]]
  [ ! -e "$TEST_ROOT/bare.git/.development-system.toml" ]
}

@test "setup reports a stable error when an option value is missing" {
  run "$REPO_ROOT/plugins/development-system/bin/development-system" setup --project

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.missing_option_value option=--project"* ]]
}

@test "setup does not consume another option as a missing value" {
  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project \
    --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.missing_option_value option=--project"* ]]
}

@test "personal-trunk setup previews without mutation and applies one commit" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -m "test: initialize fixture"
  before_head="$(git -C "$TEST_ROOT/project" rev-parse HEAD)"
  before_status="$(git -C "$TEST_ROOT/project" status --porcelain=v1)"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"write .development-system.toml"* ]]
  [ ! -e "$TEST_ROOT/project/.development-system.toml" ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
  [ "$(git -C "$TEST_ROOT/project" status --porcelain=v1)" = "$before_status" ]

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --apply \
    --yes

  [ "$status" -eq 0 ]
  [ -f "$TEST_ROOT/project/.development-system.toml" ]
  grep -Fq 'mode = "concurrent-tickets"' "$TEST_ROOT/project/.development-system.toml"
  [ "$(git -C "$TEST_ROOT/project" rev-list --count HEAD)" -eq 2 ]
  [ "$(git -C "$TEST_ROOT/project" -c log.showSignature=false log -1 --format=%s)" = "chore: initialize development system" ]
}

@test "setup reconciles explicitly enabled Beads tools without mutating an existing policy" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  mkdir -p "$TEST_ROOT/project/.beads"
  printf '%s\n' \
    'schema_version = 2' \
    '' \
    '[features]' \
    'beads = true' \
    >"$TEST_ROOT/project/.development-system.toml"
  printf 'project-owned Beads state\n' >"$TEST_ROOT/project/.beads/project-state"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md .development-system.toml .beads/project-state
  git -C "$TEST_ROOT/project" -c commit.gpgSign=false commit -m "test: initialize configured fixture"
  printf 'uncommitted project change\n' >"$TEST_ROOT/project/README.md"
  before_head="$(git -C "$TEST_ROOT/project" rev-parse HEAD)"
  before_status="$(git -C "$TEST_ROOT/project" status --porcelain=v1)"
  before_config="$(cat "$TEST_ROOT/project/.development-system.toml")"
  before_beads_state="$(cat "$TEST_ROOT/project/.beads/project-state")"
  node_bin="$(command -v node)"
  tool_invocations="$TEST_ROOT/tool-invocations"
  mkdir -p "$TEST_ROOT/test-bin"
  cat >"$TEST_ROOT/test-bin/node" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" ]]; then
  case "${2:-}" in
    status)
      printf 'status\n' >>"$SETUP_TOOL_INVOCATIONS"
      printf '%s\n' '{"schemaVersion":1,"destination":"/fixture/user-global","installationScope":"user-global","requiresSudo":false,"inheritedPathIncludesDestination":true,"usesUserGlobal":false,"pathAction":null,"tools":[{"name":"bd","requiredFor":["beads"],"targetVersion":"1.1.2","status":"missing","currentVersion":null,"executable":null,"source":null}]}'
      ;;
    install)
      printf 'install\n' >>"$SETUP_TOOL_INVOCATIONS"
      printf '%s\n' '{"schemaVersion":1,"destination":"/fixture/user-global","installationScope":"user-global","requiresSudo":false,"inheritedPathIncludesDestination":true,"usesUserGlobal":true,"pathAction":null,"installed":["bd"],"tools":[{"name":"bd","requiredFor":["beads"],"targetVersion":"1.1.2","status":"compatible","currentVersion":"1.1.2","executable":"/fixture/user-global/bd","source":"user-global"}]}'
      ;;
    *)
      exit 87
      ;;
  esac
  exit 0
fi

exec "$REAL_NODE" "$@"
SH
  chmod +x "$TEST_ROOT/test-bin/node"

  run env \
    REAL_NODE="$node_bin" \
    SETUP_TOOL_INVOCATIONS="$tool_invocations" \
    PATH="$TEST_ROOT/test-bin:$PATH" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --enable beads \
    --apply \
    --yes

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.setup_no_changes"* ]]
  [[ "$output" == *"development_system.setup_tools_installed bd=1.1.2"* ]]
  [[ "$output" != *"development_system.setup_applied"* ]]
  [ "$(cat "$tool_invocations")" = $'status\ninstall' ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
  [ "$(git -C "$TEST_ROOT/project" status --porcelain=v1)" = "$before_status" ]
  [ "$(cat "$TEST_ROOT/project/.development-system.toml")" = "$before_config" ]
  [ "$(cat "$TEST_ROOT/project/.beads/project-state")" = "$before_beads_state" ]

  : >"$TEST_ROOT/project/README.md"

  run env \
    REAL_NODE="$node_bin" \
    SETUP_TOOL_INVOCATIONS="$tool_invocations" \
    PATH="$TEST_ROOT/test-bin:$PATH" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.config_already_exists"* ]]
  [ "$(cat "$tool_invocations")" = $'status\ninstall\nstatus' ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
  [ -z "$(git -C "$TEST_ROOT/project" status --porcelain=v1)" ]

  run env \
    REAL_NODE="$node_bin" \
    SETUP_TOOL_INVOCATIONS="$tool_invocations" \
    PATH="$TEST_ROOT/test-bin:$PATH" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --enable beads \
    --enable beads \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.config_already_exists"* ]]

  run env \
    REAL_NODE="$node_bin" \
    SETUP_TOOL_INVOCATIONS="$tool_invocations" \
    PATH="$TEST_ROOT/test-bin:$PATH" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --enable beads \
    --disable beads \
    --enable beads \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.config_already_exists"* ]]
  [ "$(cat "$tool_invocations")" = $'status\ninstall\nstatus\nstatus\nstatus' ]

  printf '%s\n' \
    'note = """' \
    '[features]' \
    'beads = true' \
    '"""' \
    >"$TEST_ROOT/project/.development-system.toml"
  git -C "$TEST_ROOT/project" add .development-system.toml
  git -C "$TEST_ROOT/project" -c commit.gpgSign=false commit -m "test: add multiline string fixture"

  run env \
    REAL_NODE="$node_bin" \
    SETUP_TOOL_INVOCATIONS="$tool_invocations" \
    PATH="$TEST_ROOT/test-bin:$PATH" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --enable beads \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.config_already_exists"* ]]
  [ "$(cat "$tool_invocations")" = $'status\ninstall\nstatus\nstatus\nstatus\nstatus' ]
}

@test "setup preserves an unrelated file resembling its temporary config" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -m "test: initialize fixture"
  printf 'owned by user\n' >"$TEST_ROOT/project/.development-system.toml.development-system-tmp"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --apply \
    --yes

  [ "$status" -eq 0 ]
  [ "$(cat "$TEST_ROOT/project/.development-system.toml.development-system-tmp")" = "owned by user" ]
}

@test "setup refuses to replace a dangling config symlink" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -m "test: initialize fixture"
  ln -s missing-target "$TEST_ROOT/project/.development-system.toml"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.config_already_exists"* ]]
  [ -L "$TEST_ROOT/project/.development-system.toml" ]
  [ "$(readlink "$TEST_ROOT/project/.development-system.toml")" = "missing-target" ]
}

@test "setup rolls back its config and index when the initialization commit fails" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -m "test: initialize fixture"
  real_git="$(command -v git)"
  mkdir -p "$TEST_ROOT/test-bin"
  cat >"$TEST_ROOT/test-bin/git" <<'SH'
#!/bin/sh
case "$*" in
  *"chore: initialize development system"*) exit 87 ;;
esac
exec "$REAL_GIT" "$@"
SH
  chmod +x "$TEST_ROOT/test-bin/git"
  before_head="$(git -C "$TEST_ROOT/project" rev-parse HEAD)"

  run env HOME="$TEST_ROOT/home" REAL_GIT="$real_git" PATH="$TEST_ROOT/test-bin:$PATH" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --apply \
    --yes

  [ "$status" -ne 0 ]
  [ ! -e "$TEST_ROOT/project/.development-system.toml" ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
  [ -z "$(git -C "$TEST_ROOT/project" status --porcelain=v1)" ]
}

@test "doctor warns about conflicting plugins settings and user-managed MCPs" {
  mkdir -p \
    "$TEST_ROOT/project/.claude" \
    "$TEST_ROOT/project/.codex" \
    "$TEST_ROOT/home/.claude/plugins" \
    "$TEST_ROOT/home/.codex"
  touch "$TEST_ROOT/project/.development-system.toml"
  printf '%s\n' '{"enabledPlugins":{"development-system@ai-plugins":true,"context7@context7-marketplace":true,"hindsight-memory@hindsight":true,"another-plugin@third-party":true}}' \
    >"$TEST_ROOT/project/.claude/settings.json"
  printf '%s\n' '{"plugins":{"context7@context7-marketplace":[{"version":"1.0.0"}],"hindsight-memory@hindsight":[{"version":"1.0.0"}],"another-plugin@third-party":[{"version":"1.0.0"}]}}' \
    >"$TEST_ROOT/home/.claude/plugins/installed_plugins.json"
  printf '%s\n' \
    '[features]' \
    'hooks = false' \
    'allow_managed_hooks_only = true' \
    '[plugins."context7@context7-marketplace"]' \
    'enabled = true' \
    '[plugins.another-plugin]' \
    'enabled = true' \
    >"$TEST_ROOT/project/.codex/config.toml"
  printf '%s\n' '{"mcpServers":{"custom":{"command":"custom-mcp"}}}' \
    >"$TEST_ROOT/project/.mcp.json"

  printf '%s\n' '[plugins."another-global@marketplace"]' 'enabled = true' \
    >"$TEST_ROOT/home/.codex/config.toml"

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project"

  [ "$status" -eq 0 ]
  [[ "$output" == *"conflicting_plugins harness=claude"* ]]
  [[ "$output" == *"conflicting_plugins harness=codex"* ]]
  [[ "$output" == *"setting=hooks_disabled"* ]]
  [[ "$output" == *"setting=managed_hooks_only"* ]]
  [[ "$output" == *"user_managed_mcps_detected"* ]]
  [[ "$output" == *"supply_chain_recommendation"* ]]
}

@test "doctor permits the official Context7 and Hindsight adjunct integrations" {
  mkdir -p \
    "$TEST_ROOT/project/.claude" \
    "$TEST_ROOT/project/.codex" \
    "$TEST_ROOT/home/.claude/plugins" \
    "$TEST_ROOT/home/.codex"
  touch "$TEST_ROOT/project/.development-system.toml"
  printf '%s\n' '{"enabledPlugins":{"development-system@ai-plugins":true,"context7@context7-marketplace":true,"hindsight-memory@hindsight":true}}' \
    >"$TEST_ROOT/project/.claude/settings.json"
  printf '%s\n' '{"plugins":{"development-system@ai-plugins":[{"version":"1.0.0"}],"context7@context7-marketplace":[{"version":"1.0.0"}],"hindsight-memory@hindsight":[{"version":"1.0.0"}]}}' \
    >"$TEST_ROOT/home/.claude/plugins/installed_plugins.json"
  printf '%s\n' \
    '[plugins."development-system@ai-plugins"]' \
    'enabled = true' \
    '[plugins."context7@context7-marketplace"]' \
    'enabled = true' \
    >"$TEST_ROOT/project/.codex/config.toml"

  run env HOME="$TEST_ROOT/home" \
    CLAUDE_CONFIG_DIR="$TEST_ROOT/home/.claude" \
    CODEX_HOME="$TEST_ROOT/home/.codex" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project"

  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "doctor permits adjunct integrations only from their official marketplaces" {
  mkdir -p \
    "$TEST_ROOT/project/.claude" \
    "$TEST_ROOT/project/.codex" \
    "$TEST_ROOT/home/.claude/plugins" \
    "$TEST_ROOT/home/.codex"
  touch "$TEST_ROOT/project/.development-system.toml"
  printf '%s\n' '{"enabledPlugins":{"context7@untrusted-marketplace":true,"hindsight-memory@untrusted-marketplace":true}}' \
    >"$TEST_ROOT/project/.claude/settings.json"
  printf '%s\n' '{"plugins":{"context7@untrusted-marketplace":[{"version":"1.0.0"}],"hindsight-memory@untrusted-marketplace":[{"version":"1.0.0"}]}}' \
    >"$TEST_ROOT/home/.claude/plugins/installed_plugins.json"
  printf '%s\n' '[plugins."context7@untrusted-marketplace"]' 'enabled = true' \
    >"$TEST_ROOT/project/.codex/config.toml"

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project"

  [ "$status" -eq 0 ]
  [[ "$output" == *"conflicting_plugins harness=claude"* ]]
  [[ "$output" == *"conflicting_plugins harness=codex"* ]]
}

@test "doctor reports only conflicts for the selected harness" {
  mkdir -p \
    "$TEST_ROOT/project/.claude" \
    "$TEST_ROOT/project/.codex" \
    "$TEST_ROOT/home/.claude/plugins" \
    "$TEST_ROOT/home/.codex"
  touch "$TEST_ROOT/project/.development-system.toml"
  printf '%s\n' '{"enabledPlugins":{"another-plugin@third-party":true}}' \
    >"$TEST_ROOT/project/.claude/settings.json"
  printf '%s\n' '{"plugins":{"another-plugin@third-party":[{"version":"1.0.0"}]}}' \
    >"$TEST_ROOT/home/.claude/plugins/installed_plugins.json"
  printf '%s\n' '[plugins.another-plugin]' 'enabled = true' \
    >"$TEST_ROOT/project/.codex/config.toml"
  printf '%s\n' '{"mcpServers":{"custom":{"command":"custom-mcp"}}}' \
    >"$TEST_ROOT/project/.mcp.json"

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project" \
    --harness pi

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.unsupported_harness harness=pi"* ]]

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project" \
    --harness claude

  [ "$status" -eq 0 ]
  [[ "$output" == *"conflicting_plugins harness=claude"* ]]
  [[ "$output" == *"user_managed_mcps_detected harness=claude"* ]]
  [[ "$output" == *"supply_chain_recommendation harness=claude"* ]]
  [[ "$output" != *"harness=codex"* ]]

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project" \
    --harness codex

  [ "$status" -eq 0 ]
  [[ "$output" == *"conflicting_plugins harness=codex"* ]]
  [[ "$output" == *"supply_chain_recommendation harness=codex"* ]]
  [[ "$output" != *"harness=claude"* ]]
  [[ "$output" != *"user_managed_mcps_detected"* ]]
}

@test "doctor rejects an unknown harness instead of silently skipping checks" {
  mkdir -p "$TEST_ROOT/project"
  touch "$TEST_ROOT/project/.development-system.toml"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project" \
    --harness clauude

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.unsupported_harness harness=clauude"* ]]
}

@test "doctor is quiet outside configured projects" {
  mkdir -p "$TEST_ROOT/project" "$TEST_ROOT/home"
  printf '%s\n' '{"mcpServers":{"custom":{"command":"custom-mcp"}}}' \
    >"$TEST_ROOT/project/.mcp.json"

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project"

  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "Codex session start emits one valid warning object" {
  mkdir -p "$TEST_ROOT/project/.codex" "$TEST_ROOT/home"
  touch "$TEST_ROOT/project/.development-system.toml"
  printf '%s\n' '[features]' 'hooks = false' \
    >"$TEST_ROOT/project/.codex/config.toml"

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    session-start \
    --project "$TEST_ROOT/project" \
    --harness codex \
    --format json

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e \
    '.continue == true and (.systemMessage | contains("hooks_disabled"))' \
    >/dev/null
}

@test "session start records an opt-in eval marker without adding normal output" {
  mkdir -p "$TEST_ROOT/project" "$TEST_ROOT/home"
  marker="$TEST_ROOT/session-start.marker"

  run env \
    HOME="$TEST_ROOT/home" \
    DEVELOPMENT_SYSTEM_EVAL_SESSION_START_MARKER="$marker" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    session-start \
    --project "$TEST_ROOT/project" \
    --harness claude

  [ "$status" -eq 0 ]
  [ -z "$output" ]
  [ -f "$marker" ]
}

@test "integration contract gives explicit safe Codex and Claude handoffs" {
  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    integrations \
    --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"integrations --harness all|claude|codex"* ]]
  [[ "$output" == *"Beads hooks are plugin-managed"* ]]

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    integrations \
    --harness claude

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.integration_contract harness=claude"* ]]
  [[ "$output" == *'"${CLAUDE_PLUGIN_ROOT}/bin/run-beads-hook" --project "$PWD" -- prime --hook-json'* ]]
  [[ "$output" == *"feature_gate=enabled_project_config"* ]]
  [[ "$output" == *"claude plugin marketplace add upstash/context7"* ]]
  [[ "$output" == *"claude plugin install context7@context7-marketplace"* ]]
  [[ "$output" == *"context7 credentials=owner_provided_CONTEXT7_API_KEY_in_user_environment_or_config_never_repository_or_plugin_config"* ]]
  [[ "$output" == *"claude plugin marketplace add vectorize-io/hindsight"* ]]
  [[ "$output" == *"claude plugin install hindsight-memory"* ]]
  [[ "$output" == *"hindsight retention=owner_selects_memory_scope_and_retention_policy"* ]]
  [[ "$output" == *"credentials=not_read_or_written_by_development_system"* ]]
  [[ "$output" != *"get-codex"* ]]

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    integrations \
    --harness codex

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.integration_contract harness=codex"* ]]
  [[ "$output" == *'"${PLUGIN_ROOT}/bin/run-beads-hook" --project "$PWD" -- codex-hook SessionStart'* ]]
  [[ "$output" == *'"${PLUGIN_ROOT}/bin/run-beads-hook" --project "$PWD" -- codex-hook PreCompact'* ]]
  [[ "$output" == *'"${PLUGIN_ROOT}/bin/run-beads-hook" --project "$PWD" -- codex-hook PostCompact'* ]]
  [[ "$output" == *'"${PLUGIN_ROOT}/bin/run-beads-hook" --project "$PWD" -- codex-hook UserPromptSubmit'* ]]
  [[ "$output" == *"feature_gate=enabled_project_config"* ]]
  [[ "$output" == *"open_/hooks_inspect_and_explicitly_trust_the_current_Development_System_definition"* ]]
  [[ "$output" == *"do_not_use_--dangerously-bypass-hook-trust_merely_to_skip_review"* ]]
  [[ "$output" == *"codex plugin marketplace add upstash/context7"* ]]
  [[ "$output" == *"codex plugin add context7@context7-marketplace"* ]]
  [[ "$output" == *"context7 credentials=owner_provided_CONTEXT7_API_KEY_in_user_environment_or_config_never_repository_or_plugin_config"* ]]
  [[ "$output" == *"curl -fsSL https://hindsight.vectorize.io/get-codex | bash"* ]]
  [[ "$output" == *"hindsight installer=owner_run_interactive"* ]]
  [[ "$output" == *"not_executed_by_development_system=true"* ]]
  [[ "$output" == *"hindsight remote_installer=owner_review_and_manual_execution_required"* ]]
  [[ "$output" == *'hindsight safe_owner_route="fetch_to_a_temporary_file_and_inspect_before_owner_approved_execution; never_directly_pipe_remote_code_to_bash"'* ]]
  [[ "$output" == *'hindsight hooks_file="~/.codex/hooks.json" installer_overwrites_existing_file=true'* ]]
  [[ "$output" == *'hindsight hooks_safety="if_existing_back_up_hooks_json_before_install; after_install_review_generated_replacement; merge_Hindsight_hooks_and_backup_non_Beads_shared_hooks_into_combined_file; omit_legacy_bd_Beads_lifecycle_entries"'* ]]
  [[ "$output" == *'hindsight hooks_trust="review_and_explicitly_trust_resulting_Hindsight_user_hook_definitions_through_/hooks"'* ]]
  [[ "$output" == *'hindsight uninstall_safety="do_not_run_--uninstall_when_shared_hooks_exist; preserve_backup_and_manually_remove_only_Hindsight_hooks"'* ]]
  [[ "$output" == *"hindsight retention=owner_selects_memory_scope_and_retention_policy"* ]]
  [[ "$output" == *"credentials=not_read_or_written_by_development_system"* ]]
  [ ! -e "$TEST_ROOT/home" ]

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    integrations \
    --harness pi

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.unsupported_harness harness=pi"* ]]
}

@test "Claude and Codex hooks use the feature-gated verified package Beads launcher" {
  run jq -e '
    [.hooks.SessionStart[].hooks[].command] |
      any(. == "\"${CLAUDE_PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- prime --hook-json")
  ' "$REPO_ROOT/plugins/development-system/hooks/hooks.json"
  [ "$status" -eq 0 ]

  run jq -e '
    ([.hooks.SessionStart[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook SessionStart")) and
    ([.hooks.PreCompact[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook PreCompact")) and
    ([.hooks.PostCompact[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook PostCompact")) and
    ([.hooks.UserPromptSubmit[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/run-beads-hook\" --project \"$PWD\" -- codex-hook UserPromptSubmit"))
  ' "$REPO_ROOT/plugins/development-system/hooks/codex.json"
  [ "$status" -eq 0 ]
}

@test "setup records explicit delivery and feature selections" {
  git -C "$TEST_ROOT" init --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -m "test: initialize fixture"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --delivery pull-request \
    --disable worktrees \
    --enable agentic-systems \
    --apply \
    --yes

  [ "$status" -eq 0 ]
  grep -Fq 'mode = "pull-request"' "$TEST_ROOT/project/.development-system.toml"
  grep -Fq 'worktrees = false' "$TEST_ROOT/project/.development-system.toml"
  grep -Fq 'agentic_systems = true' "$TEST_ROOT/project/.development-system.toml"
  ! grep -Fq '[mcps]' "$TEST_ROOT/project/.development-system.toml"
}
