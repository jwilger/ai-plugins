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
  [ "$(git -C "$TEST_ROOT/project" rev-list --count HEAD)" -eq 2 ]
  [ "$(git -C "$TEST_ROOT/project" -c log.showSignature=false log -1 --format=%s)" = "chore: initialize development system" ]
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

  run env REAL_GIT="$real_git" PATH="$TEST_ROOT/test-bin:$PATH" \
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
  printf '%s\n' '{"enabledPlugins":{"development-system@ai-plugins":true,"another-plugin@third-party":true}}' \
    >"$TEST_ROOT/project/.claude/settings.json"
  printf '%s\n' '{"plugins":{"another-plugin@third-party":[{"version":"1.0.0"}]}}' \
    >"$TEST_ROOT/home/.claude/plugins/installed_plugins.json"
  printf '%s\n' \
    '[features]' \
    'hooks = false' \
    'allow_managed_hooks_only = true' \
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

  [ "$status" -eq 0 ]
  [ -z "$output" ]

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

@test "Claude and Codex hooks use the verified package Beads launcher" {
  run jq -e '
    [.hooks.SessionStart[].hooks[].command] |
      any(. == "\"${CLAUDE_PLUGIN_ROOT}/bin/bd\" prime --hook-json")
  ' "$REPO_ROOT/plugins/development-system/hooks/hooks.json"
  [ "$status" -eq 0 ]

  run jq -e '
    ([.hooks.SessionStart[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/bd\" codex-hook SessionStart")) and
    ([.hooks.PreCompact[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/bd\" codex-hook PreCompact")) and
    ([.hooks.PostCompact[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/bd\" codex-hook PostCompact")) and
    ([.hooks.UserPromptSubmit[].hooks[].command] | any(. == "\"${PLUGIN_ROOT}/bin/bd\" codex-hook UserPromptSubmit"))
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
