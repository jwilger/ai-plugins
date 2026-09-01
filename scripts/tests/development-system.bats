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

@test "personal-trunk setup previews without mutation and directs apply through MCP" {
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
  [[ "$output" == *"use setup.preview followed by setup.apply confirmed=true through development-discipline MCP"* ]]
  [ ! -e "$TEST_ROOT/project/.development-system.toml" ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
  [ "$(git -C "$TEST_ROOT/project" status --porcelain=v1)" = "$before_status" ]

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup \
    --project "$TEST_ROOT/project" \
    --preset personal-trunk \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.setup_mcp_required"* ]]
  [ ! -e "$TEST_ROOT/project/.development-system.toml" ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
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

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.setup_mcp_required"* ]]
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
  [[ "$output" == *"development_system.setup_mcp_required"* ]]
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
  mkdir -p "$TEST_ROOT/project/.git/hooks"
  printf '#!/usr/bin/env bash\nexit 1\n' >"$TEST_ROOT/project/.git/hooks/pre-commit"
  chmod +x "$TEST_ROOT/project/.git/hooks/pre-commit"
  before_head="$(git -C "$TEST_ROOT/project" rev-parse HEAD)"

  run "$REPO_ROOT/plugins/development-system/bin/development-system" \
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

@test "doctor warns about conflicting Codex plugin settings" {
  mkdir -p \
    "$TEST_ROOT/project/.codex" \
    "$TEST_ROOT/home/.codex"
  touch "$TEST_ROOT/project/.development-system.toml"
  printf '%s\n' \
    '[features]' \
    'hooks = false' \
    'allow_managed_hooks_only = true' \
    '[plugins.another-plugin]' \
    'enabled = true' \
    >"$TEST_ROOT/project/.codex/config.toml"
  printf '%s\n' '[plugins."another-global@marketplace"]' 'enabled = true' \
    >"$TEST_ROOT/home/.codex/config.toml"

  run env HOME="$TEST_ROOT/home" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    doctor \
    --project "$TEST_ROOT/project"

  [ "$status" -eq 0 ]
  [[ "$output" == *"conflicting_plugins harness=codex"* ]]
  [[ "$output" == *"setting=hooks_disabled"* ]]
  [[ "$output" == *"setting=managed_hooks_only"* ]]
  [[ "$output" == *"supply_chain_recommendation"* ]]
}

@test "doctor is quiet outside configured projects" {
  mkdir -p "$TEST_ROOT/project" "$TEST_ROOT/home"
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
    --format json

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e \
    '.continue == true and (.systemMessage | contains("hooks_disabled"))' \
    >/dev/null
}

@test "legacy setup flags are previewed but apply is delegated to MCP" {
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
    --enable agentic-systems \
    --apply \
    --yes

  [ "$status" -eq 2 ]
  [[ "$output" == *"features tiber=true agentic_systems=true eval_case_reporting=false"* ]]
  [[ "$output" == *"development_system.setup_mcp_required"* ]]
  [ ! -e "$TEST_ROOT/project/.development-system.toml" ]
}

@test "tiber launcher refuses use when the project disables Tiber" {
  git -C "$TEST_ROOT" init --initial-branch=main disabled-tiber
  git -C "$TEST_ROOT/disabled-tiber" config user.email test@example.com
  git -C "$TEST_ROOT/disabled-tiber" config user.name "Test User"
  touch "$TEST_ROOT/disabled-tiber/README.md"
  git -C "$TEST_ROOT/disabled-tiber" add README.md
  git -C "$TEST_ROOT/disabled-tiber" commit -m "test: initialize fixture"
  printf '%s\n' \
    'schema_version = 3' \
    '' \
    '[features]' \
    'tiber = false' \
    >"$TEST_ROOT/disabled-tiber/.development-system.toml"

  run bash -c \
    'cd "$1" && "$2" list' \
    _ \
    "$TEST_ROOT/disabled-tiber" \
    "$REPO_ROOT/plugins/development-system/bin/tiber"

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.feature_disabled feature=tiber"* ]]
}
