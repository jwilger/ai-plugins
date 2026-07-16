#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  git_common_dir="$(cd "$ROOT" && cd "$(git rev-parse --git-common-dir)" && pwd -P)"
  MAIN_CHECKOUT="$(cd "$git_common_dir/.." && pwd -P)"
  RUNNER="$ROOT/scripts/evals/run.sh"
  SIGNAL_FIXTURE_ROOT=""
  SIGNAL_RUNNER_PID=""
  SIGNAL_EVAL_PGID=""
  SIGNAL_CHILD_PID=""
  SIGNAL_GRANDCHILD_PID=""
}

copy_eval_runner() {
  destination="$1"
  cp "$RUNNER" "$destination"
  cp \
    "$ROOT/scripts/evals/provider-compositions.mjs" \
    "${destination%/*}/provider-compositions.mjs"
}

teardown() {
  [ -z "$SIGNAL_EVAL_PGID" ] || kill -KILL -- "-$SIGNAL_EVAL_PGID" 2>/dev/null || true
  if [ -n "$SIGNAL_RUNNER_PID" ]; then
    kill -KILL -- "-$SIGNAL_RUNNER_PID" 2>/dev/null || true
    kill -KILL "$SIGNAL_RUNNER_PID" 2>/dev/null || true
    wait "$SIGNAL_RUNNER_PID" 2>/dev/null || true
  fi
  [ -z "$SIGNAL_CHILD_PID" ] || kill -KILL "$SIGNAL_CHILD_PID" 2>/dev/null || true
  [ -z "$SIGNAL_GRANDCHILD_PID" ] || kill -KILL "$SIGNAL_GRANDCHILD_PID" 2>/dev/null || true
  [ -z "$SIGNAL_FIXTURE_ROOT" ] || rm -rf "$SIGNAL_FIXTURE_ROOT"
}

@test "eval runner prints help" {
  run "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage: scripts/evals/run.sh"* ]]
  [[ "$output" == *"Claude Code: provider=anthropic:claude-agent-sdk, model=sonnet, skills=all"* ]]
  [[ "$output" == *"Codex:       provider=openai:codex-sdk, model=gpt-5.6-terra, model_reasoning_effort=medium"* ]]
  [[ "$output" == *"CODEX_GRADER_MODEL            (default: gpt-5.6-sol)"* ]]
  [[ "$output" == *"CODEX_GRADER_REASONING_EFFORT (default: high)"* ]]
  [[ "$output" == *"Each provider loads the relevant marketplace surface for its harness"* ]]
  [[ "$output" == *"Pinned eval packages are managed by package.json and package-lock.json"* ]]
  [[ "$output" == *"@openai/codex-sdk"* ]]
  [[ "$output" == *"@anthropic-ai/claude-agent-sdk"* ]]
  [[ "$output" == *"Requires working Claude Code and Codex model authentication"* ]]
  [[ "$output" == *"Prompt response caching and hosted sharing are disabled"* ]]
  [[ "$output" == *"EVAL_PROVIDER_FILTER"* ]]
  [[ "$output" == *"PROMPTFOO_MAX_CONCURRENCY    (allowed: 1-2; default: 1)"* ]]
  [[ "$output" == *"EVAL_TIMEOUT                 (default: 90m for full behavior runs, 20m otherwise;"* ]]
  [[ "$output" == *"EVAL_TIMEOUT_FULL_DEFAULT    (default: 90m)"* ]]
  [[ "$output" == *"EVAL_TIMEOUT_FOCUSED_DEFAULT (default: 20m)"* ]]
  [[ "$output" == *"set to 0 to disable)"* ]]
  [[ "$output" == *"EVAL_TIMEOUT_KILL_AFTER      (default: 30s; force-kill grace period)"* ]]
  [[ "$output" == *"EVAL_INTERRUPT_GRACE         (default: 2s between INT, TERM, and KILL)"* ]]
  [[ "$output" == *"EVAL_OUT_DIR                 (default: evals/out; isolates generated config and artifacts)"* ]]
  [[ "$output" == *"results.junit.xml"* ]]
}

@test "eval runner dry-run uses provider-backed harness config and repo-owned artifacts" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/evals/ensure-node-deps.sh"* ]]
  [[ "$output" == *"timeout --kill-after 30s 90m"* ]]
  [[ "$output" == *"node_modules/.bin/promptfoo"* ]]
  [[ "$output" != *"npx --yes"* ]]
  [[ "$output" == *"--max-concurrency 1"* ]]
  [[ "$output" == *"--no-cache"* ]]
  [[ "$output" == *"--no-share"* ]]
  [[ "$output" == *"evals/out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [[ "$output" == *"evals/out/results.json"* ]]
  [[ "$output" == *"evals/out/report.html"* ]]
  [[ "$output" == *"evals/out/results.junit.xml"* ]]
}

@test "eval runner rejects concurrency above the canonical cap before printing a Promptfoo launch" {
  run env PROMPTFOO_MAX_CONCURRENCY=3 "$RUNNER" --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"PROMPTFOO_MAX_CONCURRENCY must be 1 or 2; got 3"* ]]
  [[ "$output" != *"promptfoo eval"* ]]
}

@test "eval runner dry-run uses repo-owned generated paths from outside repo cwd" {
  other_cwd="$(mktemp -d)"

  run bash -c 'cd "$1" && "$2" --dry-run' _ "$other_cwd" "$RUNNER"

  rm -rf "$other_cwd"
  [ "$status" -eq 0 ]
  [[ "$output" == *"$ROOT/evals/out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [[ "$output" != *"$other_cwd/evals/out/generated"* ]]
}

@test "eval runner resolves a relative output directory from the caller directory" {
  other_cwd="$(mktemp -d)"
  relative_out="relative-output-$BATS_TEST_NUMBER-$$"

  run bash -c '
    cd "$1"
    EVAL_OUT_DIR="$2" "$3" --dry-run
  ' _ "$other_cwd" "$relative_out" "$RUNNER"

  rm -rf "$ROOT/$relative_out"
  rm -rf "$other_cwd"
  [ "$status" -eq 0 ]
  [[ "$output" == *"$other_cwd/$relative_out/results.json"* ]]
  [[ "$output" != *"$ROOT/$relative_out/results.json"* ]]
}

@test "eval runner dry-run supports an isolated output directory" {
  isolated_out="$(mktemp -d)/benchmark-output"

  run env EVAL_OUT_DIR="$isolated_out" "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"$isolated_out/results.json"* ]]
  [[ "$output" == *"$isolated_out/report.html"* ]]
  [[ "$output" == *"$isolated_out/results.junit.xml"* ]]
  [[ "$output" == *"$isolated_out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [ ! -e "$isolated_out" ]

  rm -rf "${isolated_out%/*}"
}

@test "eval runner dry-run leaves an empty custom output directory unclaimed" {
  temp_root="$(mktemp -d)"
  isolated_out="$temp_root/benchmark-output"
  mkdir "$isolated_out"
  chmod 0711 "$isolated_out"
  original_inode="$(stat -c %i "$isolated_out")"

  run env EVAL_OUT_DIR="$isolated_out" "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [ "$(stat -c %i "$isolated_out")" = "$original_inode" ]
  [ "$(stat -c %a "$isolated_out")" = "711" ]
  [ ! -e "$isolated_out/.ai-plugins-eval-output" ]

  rm -rf "$temp_root"
}

@test "eval runner refuses a nonempty unowned custom output before generated writes" {
  temp_root="$(mktemp -d)"
  isolated_out="$temp_root/benchmark-output"
  mkdir "$isolated_out"
  printf 'keep me\n' >"$isolated_out/user-file"

  run env EVAL_OUT_DIR="$isolated_out" "$RUNNER" --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"refusing unowned eval output directory"* ]]
  grep -q 'keep me' "$isolated_out/user-file"
  [ ! -e "$isolated_out/generated" ]

  rm -rf "$temp_root"
}

@test "eval runner accepts a legacy nested directory under the repo eval output root" {
  nested_out="$ROOT/evals/out/owned-nested-$BATS_TEST_NUMBER-$$"
  mkdir -p "$nested_out"
  printf 'legacy focused result\n' >"$nested_out/results.json"

  run env EVAL_OUT_DIR="$nested_out" "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  grep -q 'legacy focused result' "$nested_out/results.json"
  [ ! -e "$nested_out/.ai-plugins-eval-output" ]
  rm -rf "$nested_out"
}

@test "eval runner identifies the repository root as a protected output path" {
  run env EVAL_OUT_DIR="$ROOT" "$RUNNER" --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"eval output path contains protected root: $ROOT"* ]]
}

@test "eval runner dry-run preserves artifacts in a marker-owned custom output" {
  temp_root="$(mktemp -d)"
  isolated_out="$temp_root/benchmark-output"
  mkdir "$isolated_out"
  printf 'ai-plugins eval output\n' >"$isolated_out/.ai-plugins-eval-output"
  printf 'results sentinel\n' >"$isolated_out/results.json"
  printf 'report sentinel\n' >"$isolated_out/report.html"
  printf 'junit sentinel\n' >"$isolated_out/results.junit.xml"
  printf 'status sentinel\n' >"$isolated_out/status.json"
  mkdir "$isolated_out/generated"
  printf 'config sentinel\n' >"$isolated_out/generated/agentic-systems-engineering.behavior.yaml"
  printf 'metadata sentinel\n' >"$isolated_out/generated/agentic-systems-engineering.behavior.metadata.json"

  run env EVAL_OUT_DIR="$isolated_out" "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  grep -q 'results sentinel' "$isolated_out/results.json"
  grep -q 'report sentinel' "$isolated_out/report.html"
  grep -q 'junit sentinel' "$isolated_out/results.junit.xml"
  grep -q 'status sentinel' "$isolated_out/status.json"
  grep -q 'config sentinel' "$isolated_out/generated/agentic-systems-engineering.behavior.yaml"
  grep -q 'metadata sentinel' "$isolated_out/generated/agentic-systems-engineering.behavior.metadata.json"

  rm -rf "$temp_root"
}

@test "eval runner serializes live provider runs and accepts only its exact inherited lock" {
  temp_root="$(mktemp -d)"
  lock_path="$MAIN_CHECKOUT/.dependencies/evals/provider-eval.lock"
  config="$temp_root/promptfooconfig.yaml"
  fake_promptfoo="$temp_root/promptfoo"
  provider_marker="$temp_root/provider-invoked"
  mkdir -p "$(dirname "$lock_path")"
  printf 'prompts: []\nproviders: []\ntests: []\n' >"$config"
  cat >"$fake_promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
touch "$PROVIDER_MARKER"
SH
  chmod +x "$fake_promptfoo"

  exec 8>>"$lock_path"
  flock --nonblock 8

  run env \
    PROMPTFOO_BIN="$fake_promptfoo" \
    PROVIDER_MARKER="$provider_marker" \
    EVAL_OUT_DIR="$temp_root/blocked-output" \
    "$RUNNER" "$config"

  [ "$status" -eq 75 ]
  [[ "$output" == *"provider-backed eval already active; lock is held: $lock_path"* ]]
  [ ! -e "$temp_root/blocked-output" ]
  [ ! -e "$provider_marker" ]

  run env \
    AI_PLUGINS_EVAL_LOCK_HELD=1 \
    AI_PLUGINS_EVAL_LOCK_PATH="$temp_root/not-the-provider-lock" \
    AI_PLUGINS_EVAL_LOCK_FD=8 \
    PROMPTFOO_BIN="$fake_promptfoo" \
    PROVIDER_MARKER="$provider_marker" \
    EVAL_OUT_DIR="$temp_root/wrong-inherited-output" \
    "$RUNNER" "$config"

  [ "$status" -eq 75 ]
  [ ! -e "$temp_root/wrong-inherited-output" ]
  [ ! -e "$provider_marker" ]

  run env \
    PROMPTFOO_BIN="$fake_promptfoo" \
    PROVIDER_MARKER="$provider_marker" \
    EVAL_OUT_DIR="$temp_root/dry-output" \
    "$RUNNER" --dry-run "$config"

  [ "$status" -eq 0 ]
  [ ! -e "$provider_marker" ]

  run env \
    AI_PLUGINS_EVAL_LOCK_HELD=1 \
    AI_PLUGINS_EVAL_LOCK_PATH="$lock_path" \
    AI_PLUGINS_EVAL_LOCK_FD=8 \
    PROMPTFOO_BIN="$fake_promptfoo" \
    PROVIDER_MARKER="$provider_marker" \
    EVAL_OUT_DIR="$temp_root/inherited-output" \
    "$RUNNER" "$config"

  flock --unlock 8
  exec 8>&-

  [ "$status" -eq 0 ]
  [ -e "$provider_marker" ]
  rm -rf "$temp_root"
}

@test "eval runner shares its provider lock across linked worktrees" {
  temp_root="$(mktemp -d)"
  fixture_main="$temp_root/main"
  fixture_worktree="$temp_root/linked"
  fake_bin="$temp_root/bin"
  config="$temp_root/promptfooconfig.yaml"
  preparation_marker="$temp_root/preparation-invoked"
  mkdir -p "$fixture_main/scripts/evals" "$fake_bin"
  copy_eval_runner "$fixture_main/scripts/evals/run.sh"
  printf 'prompts: []\nproviders: []\ntests: []\n' >"$config"

  git -C "$fixture_main" init -q
  git -C "$fixture_main" config user.name fixture
  git -C "$fixture_main" config user.email fixture@example.invalid
  git -C "$fixture_main" config commit.gpgSign false
  git -C "$fixture_main" add scripts/evals/run.sh
  git -C "$fixture_main" commit -qm fixture
  git -C "$fixture_main" worktree add -q --detach "$fixture_worktree"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'touch "$PREPARATION_MARKER"' \
    'exit 91' \
    >"$fake_bin/node"
  chmod +x "$fake_bin/node"

  lock_path="$fixture_main/.dependencies/evals/provider-eval.lock"
  mkdir -p "$(dirname "$lock_path")"
  exec 8>>"$lock_path"
  flock --nonblock 8

  run env \
    PATH="$fake_bin:$PATH" \
    PREPARATION_MARKER="$preparation_marker" \
    EVAL_OUT_DIR="$temp_root/output" \
    "$fixture_worktree/scripts/evals/run.sh" "$config"
  run_status="$status"
  run_output="$output"
  preparation_invoked=0
  [ ! -e "$preparation_marker" ] || preparation_invoked=1

  flock --unlock 8
  exec 8>&-
  rm -rf "$temp_root"

  [ "$run_status" -eq 75 ]
  [[ "$run_output" == *"provider-backed eval already active; lock is held: $lock_path"* ]]
  [ "$preparation_invoked" -eq 0 ]
}

@test "eval runner dry-run prepares targeted Codex home from selected behavior cases" {
  run env EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  targeted_line="$(printf '%s\n' "$output" | grep -- '--plugin-mode targeted-plugins')"
  [[ "$targeted_line" == *"prepare-codex-home.mjs"* ]]
  [[ "$targeted_line" == *"--plugins tiber"* ]]
  [[ "$targeted_line" != *"advisor"* ]]
  [[ "$targeted_line" != *"\\,"* ]]
}

@test "eval runner prepares a focused Codex home with exactly the selected case plugins" {
  fixture_root="$(mktemp -d)"
  fake_promptfoo="$fixture_root/promptfoo"
  targeted_home="$fixture_root/codex-targeted"
  cat >"$fake_promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_OUT_DIR="$fixture_root/out" \
    EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture \
    EVAL_PROVIDER_FILTER=codex-gpt-5.6-terra-targeted-plugins \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME="$fixture_root/codex-full" \
    CODEX_EVAL_HOME_FULL_MARKETPLACE="$fixture_root/codex-full" \
    CODEX_EVAL_HOME_NO_PLUGINS="$fixture_root/codex-none" \
    CODEX_EVAL_HOME_TARGETED_PLUGINS="$targeted_home" \
    "$RUNNER"

  [ "$status" -eq 0 ]
  [ "$(grep -c '^\[plugins\.' "$targeted_home/config.toml")" -eq 1 ]
  grep -q '\[plugins\."tiber@ai-plugins"\]' "$targeted_home/config.toml"
  ! grep -q 'advisor@ai-plugins' "$targeted_home/config.toml"
  [ -d "$targeted_home/plugins/cache/ai-plugins/tiber" ]
  [ "$(find "$targeted_home/plugins/cache/ai-plugins" -mindepth 1 -maxdepth 1 -type d | wc -l)" -eq 1 ]

  rm -rf "$fixture_root"
}

@test "eval runner rejects selected Codex home aliases and overlaps before preparing any home" {
  for layout in exact-alias symlinked-descendant case-alias-descendant; do
    fixture_root="$(mktemp -d)"
    fake_promptfoo="$fixture_root/promptfoo"
    full_home="$fixture_root/full-home"
    mkdir -p "$full_home"
    printf 'ai-plugins Codex eval home\n' >"$full_home/.ai-plugins-eval-home"
    printf 'preserve config\n' >"$full_home/config.toml"
    printf 'preserve sentinel\n' >"$full_home/sentinel"
    printf '#!/usr/bin/env bash\nexit 0\n' >"$fake_promptfoo"
    chmod +x "$fake_promptfoo"

    case "$layout" in
      exact-alias)
        targeted_home="$full_home"
        ;;
      symlinked-descendant)
        ln -s "$full_home" "$fixture_root/full-home-link"
        targeted_home="$(realpath -m --relative-to="$ROOT" "$fixture_root/full-home-link/targeted")"
        ;;
      case-alias-descendant)
        full_home="$fixture_root/CaseHome"
        mkdir -p "$full_home"
        printf 'ai-plugins Codex eval home\n' >"$full_home/.ai-plugins-eval-home"
        printf 'preserve config\n' >"$full_home/config.toml"
        printf 'preserve sentinel\n' >"$full_home/sentinel"
        targeted_home="$fixture_root/casehome/targeted"
        ;;
    esac

    run env \
      OPENAI_API_KEY=fixture \
      PROMPTFOO_BIN="$fake_promptfoo" \
      EVAL_OUT_DIR="$fixture_root/out" \
      EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture \
      EVAL_PROVIDER_FILTER=codex-gpt-5.6-terra-targeted-plugins \
      EVAL_TIMEOUT=0 \
      CODEX_EVAL_HOME="$full_home" \
      CODEX_EVAL_HOME_FULL_MARKETPLACE="$full_home" \
      CODEX_EVAL_HOME_TARGETED_PLUGINS="$targeted_home" \
      "$RUNNER"

    [ "$status" -eq 2 ]
    [[ "$output" == *"Codex eval homes overlap for incompatible compositions"* ]]
    [[ "$output" == *"full-marketplace"* ]]
    [[ "$output" == *"targeted-plugins"* ]]
    grep -q '^preserve config$' "$full_home/config.toml"
    grep -q '^preserve sentinel$' "$full_home/sentinel"
    [ ! -e "$full_home/targeted" ]

    rm -rf "$fixture_root"
  done
}

@test "eval runner ignores overlapping homes for unselected Codex modes" {
  fixture_root="$(mktemp -d)"
  fake_promptfoo="$fixture_root/promptfoo"
  shared_home="$fixture_root/shared-home"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_OUT_DIR="$fixture_root/out" \
    EVAL_PROVIDER_FILTER=codex-gpt-5.6-terra \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME="$shared_home" \
    CODEX_EVAL_HOME_FULL_MARKETPLACE="$shared_home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$shared_home" \
    CODEX_EVAL_HOME_TARGETED_PLUGINS="$shared_home" \
    "$RUNNER"

  [ "$status" -eq 0 ]
  [ -f "$shared_home/config.toml" ]

  rm -rf "$fixture_root"
}

@test "eval runner dry-run prepares only Codex grader home for Claude-only provider filter" {
  run env EVAL_PROVIDER_FILTER=claude "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"generate-config.mjs"* ]]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 1 ]
  [[ "$output" == *"--plugin-mode full-marketplace"* ]]
  [[ "$output" != *"--plugin-mode no-plugins"* ]]
  [[ "$output" != *"--plugin-mode targeted-plugins"* ]]
  [[ "$output" == *"promptfoo eval"* ]]
}

@test "eval runner dry-run prepares only selected Codex plugin mode" {
  run env EVAL_PROVIDER_FILTER=codex-gpt-5.6-terra "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 1 ]
  [[ "$output" == *"--plugin-mode full-marketplace"* ]]
  [[ "$output" != *"--plugin-mode no-plugins"* ]]
  [[ "$output" != *"--plugin-mode targeted-plugins"* ]]
}

@test "eval runner passes case filter to Promptfoo CLI" {
  run env EVAL_CASE_FILTER=tiber "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 20m"* ]]
  [[ "$output" == *"--filter-pattern tiber"* ]]
}

@test "eval runner dry-run can disable the promptfoo timeout" {
  run env EVAL_TIMEOUT=0 "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 0"* ]]
  [[ "$output" == *"node_modules/.bin/promptfoo eval"* ]]
}

@test "eval runner dry-run supports shorter local default timeout overrides" {
  run env EVAL_TIMEOUT_FULL_DEFAULT=30m EVAL_TIMEOUT_FOCUSED_DEFAULT=5m "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 30m"* ]]

  run env EVAL_TIMEOUT_FULL_DEFAULT=30m EVAL_TIMEOUT_FOCUSED_DEFAULT=5m EVAL_CASE_FILTER=tiber "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 5m"* ]]
}

@test "generated eval config can filter providers" {
  run env EVAL_PROVIDER_FILTER=claude node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: claude-code-sonnet-full-marketplace"* ]]
  [[ "$output" != *"label: codex-gpt-5.6-terra-full-marketplace"* ]]
}

@test "generated eval config exact provider variant filter selects one full-marketplace provider" {
  run env EVAL_PROVIDER_FILTER=codex-gpt-5.6-terra node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: codex-gpt-5.6-terra-full-marketplace"* ]]
  [[ "$output" != *"label: codex-gpt-5.6-terra-targeted-plugins"* ]]
  [[ "$output" != *"label: codex-gpt-5.6-terra-no-plugins"* ]]
  [[ "$output" != *"label: claude-code-sonnet"* ]]
  [[ "$output" == *"pluginModes:"*$'\n'"      - id: full-marketplace"* ]]
}

@test "generated eval config combines case and provider filters without expanding provider modes" {
  run env EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture EVAL_PROVIDER_FILTER=codex-gpt-5.6-terra node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^  - id: openai:codex-sdk$')" -eq 1 ]
  [[ "$output" == *"label: codex-gpt-5.6-terra-full-marketplace"* ]]
  [[ "$output" == *"evals/out/generated/load-harness-cases.runtime.cjs"* ]]
  [[ "$output" != *"label: codex-gpt-5.6-terra-targeted-plugins"* ]]
  [[ "$output" != *"label: codex-gpt-5.6-terra-no-plugins"* ]]
  [[ "$output" != *"label: claude-code-sonnet"* ]]
}

@test "eval runner uses project-local Promptfoo state for real runs" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'PROMPTFOO_CONFIG_DIR=%s\n' "${PROMPTFOO_CONFIG_DIR:-}"
printf 'ARGS=%s\n' "$*"
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"PROMPTFOO_CONFIG_DIR=$fixture_root/.dependencies/promptfoo"* ]]
}

@test "eval threshold checker honors case min pass rates" {
  fixture_root="$(mktemp -d)"
  results="$fixture_root/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": false,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 },
        "gradingResult": { "reason": "Stochastic rubric miss" }
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "eval threshold checker treats no-plugin misses as baseline value-gate evidence" {
  fixture_root="$(mktemp -d)"
  results="$fixture_root/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": { "label": "codex-gpt-5.6-terra-no-plugins" },
        "testCase": { "vars": { "case_id": "plugin-specific-safety", "plugin_mode": "no-plugins", "min_pass_rate": 1, "value_gate_mode": "safety-critical", "baseline_lift_threshold": 0 } },
        "gradingResult": { "pass": false, "score": 0, "reason": "No plugin-specific command known" }
      },
      {
        "provider": { "label": "codex-gpt-5.6-terra-targeted-plugins" },
        "testCase": { "vars": { "case_id": "plugin-specific-safety", "plugin_mode": "targeted-plugins", "min_pass_rate": 1, "value_gate_mode": "safety-critical", "baseline_lift_threshold": 0 } },
        "gradingResult": { "pass": true, "score": 1 }
      },
      {
        "provider": { "label": "codex-gpt-5.6-terra-full-marketplace" },
        "testCase": { "vars": { "case_id": "plugin-specific-safety", "plugin_mode": "full-marketplace", "min_pass_rate": 1, "value_gate_mode": "safety-critical", "baseline_lift_threshold": 0 } },
        "gradingResult": { "pass": true, "score": 1 }
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "eval threshold checker skips value gates when fixture marks them none" {
  fixture_root="$(mktemp -d)"
  results="$fixture_root/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": { "label": "codex-gpt-5.6-terra-full-marketplace" },
        "testCase": { "vars": { "case_id": "composition", "min_pass_rate": 1, "value_gate_mode": "none" } },
        "gradingResult": { "pass": true, "score": 1 }
      },
      {
        "provider": { "label": "codex-gpt-5.6-terra-no-plugins" },
        "testCase": { "vars": { "case_id": "composition", "min_pass_rate": 1, "value_gate_mode": "none" } },
        "gradingResult": { "pass": true, "score": 1 }
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "hard guard allows whitelisted tiber command context for task files" {
  run node - <<'NODE'
const assertHardGuards = require("./evals/promptfoo/assert-hard-guards.cjs");
const result = assertHardGuards(
  "Use `tiber list` to inspect the board instead of directly write order.md by hand.",
  { vars: { case_id: "tiber-validation-discipline" } },
);
if (!result.pass) {
  console.error(result.reason);
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "eval runner exits successfully when promptfoo sample failures meet thresholds" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": false,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 },
        "gradingResult": { "reason": "Stochastic rubric miss" }
      }
    ]
  }
}
JSON
exit 100
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "eval runner clears stale timeout status before a successful run" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin" "$fixture_root/evals/out"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/evals/out/status.json" <<'JSON'
{
  "state": "timed-out",
  "reason": "stale timeout"
}
JSON
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 1 }
      }
    ]
  }
}
JSON
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ "$status" -eq 0 ]
  [ ! -e "$fixture_root/evals/out/status.json" ]
  [[ "$output" == *"Eval thresholds passed"* ]]
  rm -rf "$fixture_root"
}

@test "eval runner writes generated runtime filter options for real generated runs" {
  fixture_bin="$(mktemp -d)"
  mkdir -p "$fixture_bin"
  cat >"$fixture_bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
cat evals/out/generated/runtime-options.json
SH
  chmod +x "$fixture_bin/promptfoo"

  run env PROMPTFOO_BIN="$fixture_bin/promptfoo" EVAL_CASE_FILTER=tiber "$RUNNER"

  rm -rf "$fixture_bin"
  rm -f "$ROOT/evals/out/generated/runtime-options.json"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"caseFilter":"tiber"'* ]]
}

@test "eval runner filtered samples use the runtime loader in an isolated output directory" {
  fixture_root="$(mktemp -d)"
  isolated_out="$fixture_root/isolated-output"
  mkdir -p "$fixture_root/bin"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
config=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-c" ]; then
    config="$2"
    break
  fi
  shift
done
runtime_loader="$EVAL_OUT_DIR/generated/load-harness-cases.runtime.cjs"
test -f "$runtime_loader"
grep -F "tests: file://$runtime_loader" "$config"
cat "$EVAL_OUT_DIR/generated/runtime-options.json"
SH
  chmod +x "$fixture_root/bin/promptfoo"

  run env \
    PROMPTFOO_BIN="$fixture_root/bin/promptfoo" \
    EVAL_OUT_DIR="$isolated_out" \
    EVAL_CASE_FILTER=tiber \
    EVAL_SAMPLES=2 \
    "$RUNNER"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"tests: file://$isolated_out/generated/load-harness-cases.runtime.cjs"* ]]
  [[ "$output" == *'"caseFilter":"tiber"'* ]]
  [[ "$output" == *'"samples":"2"'* ]]
}

@test "eval runner times out a hanging promptfoo invocation" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
sleep 5
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" EVAL_TIMEOUT=1s "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ "$status" -eq 124 ]
  [[ "$output" == *"promptfoo eval timed out after EVAL_TIMEOUT=1s"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "timed-out" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval timed out after EVAL_TIMEOUT=1s" ]
  rm -rf "$fixture_root"
}

@test "eval runner treats timeout as failure even when partial results pass thresholds" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 1 }
      }
    ]
  }
}
JSON
sleep 5
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" EVAL_TIMEOUT=1s "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ ! -e "$fixture_root/evals/out/results.json" ]
  [ -f "$fixture_root/evals/out/timeout-artifacts/"*/results.json ]
  [ "$status" -eq 124 ]
  [[ "$output" == *"promptfoo eval timed out after EVAL_TIMEOUT=1s"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "timed-out" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval timed out after EVAL_TIMEOUT=1s" ]
  [[ "$output" == *"retained partial eval artifacts in"* ]]
  [[ "$output" == *"-exit-124."* ]]
  [[ "$output" != *"Eval thresholds passed"* ]]
  rm -rf "$fixture_root"
}

@test "eval runner treats interrupted promptfoo as failure even when partial results pass thresholds" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 1 }
      }
    ]
  }
}
JSON
exit 130
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ ! -e "$fixture_root/evals/out/results.json" ]
  [ -f "$fixture_root/evals/out/timeout-artifacts/"*/results.json ]
  [ "$status" -eq 130 ]
  [[ "$output" == *"promptfoo eval was interrupted before completion with status 130"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "interrupted" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval was interrupted before completion with status 130" ]
  [[ "$output" != *"Eval thresholds passed"* ]]
  rm -rf "$fixture_root"
}

@test "eval runner records SIGINT during pre-promptfoo setup" {
  SIGNAL_FIXTURE_ROOT="$(mktemp -d)"
  fixture_root="$SIGNAL_FIXTURE_ROOT"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
on_interrupt() {
  printf 'interrupted\n' >"$PROCESS_FIXTURE_DIR/setup.interrupted"
  exit 130
}
trap on_interrupt INT
mkdir -p evals/out
printf '{"results":{"results":[]}}\n' >evals/out/results.json
printf '%s\n' "$$" >"$PROCESS_FIXTURE_DIR/setup.pid"
printf 'ready\n' >"$PROCESS_FIXTURE_DIR/setup.ready"
while true; do sleep 1; done
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'started\n' >"$PROCESS_FIXTURE_DIR/promptfoo.started"
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  setsid env --default-signal=INT \
    PROCESS_FIXTURE_DIR="$fixture_root" \
    PROMPTFOO_BIN="$fixture_root/bin/promptfoo" \
    EVAL_TIMEOUT=0 \
    "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml" \
    >"$fixture_root/runner.log" 2>&1 &
  SIGNAL_RUNNER_PID="$!"

  for _ in $(seq 1 100); do
    [ ! -s "$fixture_root/setup.ready" ] || break
    sleep 0.05
  done
  [ -s "$fixture_root/setup.ready" ]
  [ -s "$fixture_root/setup.pid" ]
  SIGNAL_CHILD_PID="$(cat "$fixture_root/setup.pid")"

  kill -INT -- "-$SIGNAL_RUNNER_PID"
  runner_exited=0
  for _ in $(seq 1 100); do
    if ! kill -0 "$SIGNAL_RUNNER_PID" 2>/dev/null; then
      runner_exited=1
      break
    fi
    sleep 0.05
  done
  [ "$runner_exited" -eq 1 ]

  runner_status=0
  wait "$SIGNAL_RUNNER_PID" || runner_status="$?"
  SIGNAL_RUNNER_PID=""

  [ "$runner_status" -eq 130 ]
  [ -f "$fixture_root/setup.interrupted" ]
  ! kill -0 "$SIGNAL_CHILD_PID" 2>/dev/null
  [ ! -e "$fixture_root/promptfoo.started" ]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "interrupted" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval was interrupted before completion with status 130" ]
  [ ! -e "$fixture_root/evals/out/results.json" ]
  [ -f "$fixture_root/evals/out/timeout-artifacts/"*/results.json ]
}

@test "eval runner forwards SIGINT received before publishing the eval pid" {
  SIGNAL_FIXTURE_ROOT="$(mktemp -d)"
  fixture_root="$SIGNAL_FIXTURE_ROOT"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/launch-hook.sh" <<'SH'
if [ -z "${EVAL_RUNNER_BASHPID:-}" ]; then
  export EVAL_RUNNER_BASHPID="$BASHPID"
fi
eval_launch_hook() {
  local command="$1"
  if [ "$BASHPID" = "$EVAL_RUNNER_BASHPID" ] && [ "$command" = 'eval_pid="$!"' ]; then
    trap - DEBUG
    for _ in {1..200}; do
      [ ! -s "$PROCESS_FIXTURE_DIR/child.ready" ] || break
      sleep 0.01
    done
    [ -s "$PROCESS_FIXTURE_DIR/child.ready" ] || exit 99
    printf 'ready\n' >"$PROCESS_FIXTURE_DIR/capture.ready"
    while [ ! -e "$PROCESS_FIXTURE_DIR/capture.release" ]; do sleep 0.01; done
  fi
}
trap 'eval_launch_hook "$BASH_COMMAND"' DEBUG
SH
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
on_interrupt() {
  printf 'interrupted\n' >"$PROCESS_FIXTURE_DIR/child.interrupted"
  exit 130
}
trap on_interrupt INT
printf '%s\n' "$$" >"$PROCESS_FIXTURE_DIR/child.pid"
printf 'ready\n' >"$PROCESS_FIXTURE_DIR/child.ready"
while true; do sleep 1; done
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  setsid env --default-signal=INT \
    BASH_ENV="$fixture_root/launch-hook.sh" \
    PROCESS_FIXTURE_DIR="$fixture_root" \
    PROMPTFOO_BIN="$fixture_root/bin/promptfoo" \
    EVAL_TIMEOUT=0 \
    EVAL_INTERRUPT_GRACE=0.1s \
    "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml" \
    >"$fixture_root/runner.log" 2>&1 &
  SIGNAL_RUNNER_PID="$!"

  for _ in $(seq 1 100); do
    [ ! -s "$fixture_root/capture.ready" ] || break
    sleep 0.05
  done
  [ -s "$fixture_root/capture.ready" ]
  [ -s "$fixture_root/child.pid" ]
  SIGNAL_CHILD_PID="$(cat "$fixture_root/child.pid")"
  SIGNAL_EVAL_PGID="$(ps -o pgid= -p "$SIGNAL_CHILD_PID" | tr -d ' ')"
  runner_pgid="$(ps -o pgid= -p "$SIGNAL_RUNNER_PID" | tr -d ' ')"
  [ "$SIGNAL_EVAL_PGID" != "$runner_pgid" ]

  kill -INT -- "-$SIGNAL_RUNNER_PID"
  touch "$fixture_root/capture.release"
  runner_exited=0
  for _ in $(seq 1 100); do
    if ! kill -0 "$SIGNAL_RUNNER_PID" 2>/dev/null; then
      runner_exited=1
      break
    fi
    sleep 0.05
  done
  [ "$runner_exited" -eq 1 ]

  runner_status=0
  wait "$SIGNAL_RUNNER_PID" || runner_status="$?"
  SIGNAL_RUNNER_PID=""

  [ "$runner_status" -eq 130 ]
  [ -f "$fixture_root/child.interrupted" ]
  ! kill -0 "$SIGNAL_CHILD_PID" 2>/dev/null
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "interrupted" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval was interrupted before completion with status 130" ]
}

@test "eval runner SIGINT terminates the complete promptfoo process group" {
  SIGNAL_FIXTURE_ROOT="$(mktemp -d)"
  fixture_root="$SIGNAL_FIXTURE_ROOT"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
printf '{"results":{"results":[]}}\n' >evals/out/results.json
grandchild_pid=""
on_interrupt() {
  printf 'interrupted\n' >"$PROCESS_FIXTURE_DIR/child.interrupted"
  [ -z "$grandchild_pid" ] || wait "$grandchild_pid" 2>/dev/null || true
  exit 130
}
trap on_interrupt INT
printf '%s\n' "$$" >"$PROCESS_FIXTURE_DIR/child.pid"
env --default-signal=INT bash -c '
  on_interrupt() {
    printf "interrupted\n" >"$PROCESS_FIXTURE_DIR/grandchild.interrupted"
  }
  trap on_interrupt INT
  trap "" TERM
  printf "%s\n" "$$" >"$PROCESS_FIXTURE_DIR/grandchild.pid"
  while true; do sleep 1; done
' &
grandchild_pid="$!"
printf 'ready\n' >"$PROCESS_FIXTURE_DIR/child.ready"
set +e
wait "$grandchild_pid"
exit "$?"
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  setsid env --default-signal=INT \
    PROCESS_FIXTURE_DIR="$fixture_root" \
    PROMPTFOO_BIN="$fixture_root/bin/promptfoo" \
    EVAL_TIMEOUT=0 \
    EVAL_TIMEOUT_KILL_AFTER=0.1s \
    EVAL_INTERRUPT_GRACE=0.1s \
    "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml" \
    >"$fixture_root/runner.log" 2>&1 &
  SIGNAL_RUNNER_PID="$!"

  for _ in $(seq 1 100); do
    [ ! -s "$fixture_root/child.ready" ] || [ ! -s "$fixture_root/grandchild.pid" ] || break
    sleep 0.05
  done
  [ -s "$fixture_root/child.ready" ]
  [ -s "$fixture_root/child.pid" ]
  [ -s "$fixture_root/grandchild.pid" ]
  SIGNAL_CHILD_PID="$(cat "$fixture_root/child.pid")"
  SIGNAL_GRANDCHILD_PID="$(cat "$fixture_root/grandchild.pid")"
  SIGNAL_EVAL_PGID="$(ps -o pgid= -p "$SIGNAL_CHILD_PID" | tr -d ' ')"
  runner_pgid="$(ps -o pgid= -p "$SIGNAL_RUNNER_PID" | tr -d ' ')"
  [ "$SIGNAL_EVAL_PGID" != "$runner_pgid" ]

  kill -INT -- "-$SIGNAL_RUNNER_PID"
  runner_exited=0
  for _ in $(seq 1 100); do
    if ! kill -0 "$SIGNAL_RUNNER_PID" 2>/dev/null; then
      runner_exited=1
      break
    fi
    sleep 0.05
  done
  [ "$runner_exited" -eq 1 ]

  runner_status=0
  wait "$SIGNAL_RUNNER_PID" || runner_status="$?"
  SIGNAL_RUNNER_PID=""

  for _ in $(seq 1 100); do
    if ! kill -0 "$SIGNAL_CHILD_PID" 2>/dev/null &&
      ! kill -0 "$SIGNAL_GRANDCHILD_PID" 2>/dev/null; then
      break
    fi
    sleep 0.05
  done

  [ "$runner_status" -eq 130 ]
  [ -f "$fixture_root/child.interrupted" ]
  [ -f "$fixture_root/grandchild.interrupted" ]
  ! kill -0 "$SIGNAL_CHILD_PID" 2>/dev/null
  ! kill -0 "$SIGNAL_GRANDCHILD_PID" 2>/dev/null
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "interrupted" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval was interrupted before completion with status 130" ]
  [ ! -e "$fixture_root/evals/out/results.json" ]
  [ -f "$fixture_root/evals/out/timeout-artifacts/"*/results.json ]
}

@test "eval runner force-kills a promptfoo process that ignores timeout termination" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/write-status.mjs" "$fixture_root/scripts/evals/write-status.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
trap '' TERM
while true; do sleep 1; done
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" EVAL_TIMEOUT=1s EVAL_TIMEOUT_KILL_AFTER=1s "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ "$status" -eq 137 ]
  [[ "$output" == *"promptfoo eval timed out after EVAL_TIMEOUT=1s"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "timed-out" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval timed out after EVAL_TIMEOUT=1s" ]
  rm -rf "$fixture_root"
}

@test "eval runner rejects invalid provider composition metadata before home preparation" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals"
  copy_eval_runner "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/generate-config.mjs" <<'NODE'
#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
const output = process.argv[process.argv.indexOf('--output') + 1];
const metadataOutput = process.argv[process.argv.indexOf('--metadata-output') + 1];
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `providers:
  - id: openai:codex-sdk
    label: codex-gpt-5.6-terra-targeted-plugins
    pluginMode: targeted-plugins
`);
const targeted = {
  label: 'codex-gpt-5.6-terra-targeted-plugins',
  provider: 'openai:codex-sdk',
  providerVariant: 'codex-gpt-5.6-terra',
  pluginMode: 'targeted-plugins',
  plugins: ['tiber'],
};
const noPlugins = {
  ...targeted,
  label: 'codex-gpt-5.6-terra-no-plugins',
  pluginMode: 'no-plugins',
  plugins: [],
};
const cases = {
  empty: [],
  duplicate: [targeted, targeted],
  inconsistent: [
    targeted,
    {
      ...targeted,
      label: 'codex-second-targeted-plugins',
      providerVariant: 'codex-second',
      plugins: ['advisor'],
    },
  ],
  targeted_empty: [{ ...targeted, plugins: [] }],
  no_plugins_nonempty: [
    {
      ...targeted,
      label: 'codex-gpt-5.6-terra-no-plugins',
      pluginMode: 'no-plugins',
    },
  ],
  missing_variant: [{ ...targeted, providerVariant: undefined }],
  unknown_provider: [{ ...targeted, provider: 'unknown:provider' }],
  unknown_mode: [
    {
      ...targeted,
      label: 'codex-gpt-5.6-terra-unknown-mode',
      pluginMode: 'unknown-mode',
    },
  ],
  label_mismatch: [{ ...targeted, label: 'mismatched-label' }],
  duplicate_plugin: [{ ...targeted, plugins: ['tiber', 'tiber'] }],
  unsorted_plugins: [{ ...targeted, plugins: ['tiber', 'advisor'] }],
  invalid_plugin_name: [{ ...targeted, plugins: ['Tiber'] }],
  missing_composition_label: [targeted],
  extra_composition_label: [targeted, noPlugins],
  both_missing_and_extra: [
    targeted,
    {
      label: 'claude-b-full-marketplace',
      provider: 'anthropic:claude-agent-sdk',
      providerVariant: 'claude-b',
      pluginMode: 'full-marketplace',
      plugins: ['advisor'],
    },
    {
      label: 'claude-c-no-plugins',
      provider: 'anthropic:claude-agent-sdk',
      providerVariant: 'claude-c',
      pluginMode: 'no-plugins',
      plugins: [],
    },
  ],
  order_insensitive: [targeted, noPlugins],
};
const providerLabelsByCase = {
  missing_composition_label: [targeted.label, noPlugins.label],
  extra_composition_label: [targeted.label],
  both_missing_and_extra: [
    'claude-z-no-plugins',
    targeted.label,
    'claude-a-full-marketplace',
  ],
  order_insensitive: [noPlugins.label, targeted.label],
};
const metadata = {
  usesCodexGrader: true,
  providerLabels: providerLabelsByCase[process.env.COMPOSITION_CASE] || [targeted.label],
};
if (process.env.COMPOSITION_CASE !== 'missing') {
  metadata.providerCompositions = cases[process.env.COMPOSITION_CASE];
}
fs.mkdirSync(path.dirname(metadataOutput), { recursive: true });
fs.writeFileSync(metadataOutput, JSON.stringify(metadata));
NODE

  for fixture in \
    "missing|generated eval metadata is missing providerCompositions" \
    "empty|providerCompositions must contain at least one provider" \
    "duplicate|duplicate provider label" \
    "inconsistent|inconsistent Codex provider compositions for targeted-plugins" \
    "targeted_empty|targeted provider composition must not be empty" \
    "no_plugins_nonempty|no-plugins provider composition must be empty" \
    "missing_variant|invalid provider composition" \
    "unknown_provider|unsupported provider in provider composition" \
    "unknown_mode|unsupported plugin mode in provider composition" \
    "label_mismatch|provider composition label does not match its variant and mode" \
    "duplicate_plugin|non-canonical plugin list" \
    "unsorted_plugins|non-canonical plugin list" \
    "invalid_plugin_name|invalid plugin list" \
    "missing_composition_label|provider composition labels do not match configured providers: missing: codex-gpt-5.6-terra-no-plugins" \
    "extra_composition_label|provider composition labels do not match configured providers: extra: codex-gpt-5.6-terra-no-plugins" \
    "both_missing_and_extra|provider composition labels do not match configured providers: missing: claude-a-full-marketplace, claude-z-no-plugins; extra: claude-b-full-marketplace, claude-c-no-plugins"; do
    composition_case="${fixture%%|*}"
    expected="${fixture#*|}"

    run env COMPOSITION_CASE="$composition_case" "$fixture_root/scripts/evals/run.sh" --dry-run

    [ "$status" -ne 0 ]
    [[ "$output" == *"$expected"* ]]
    [[ "$output" != *"prepare-codex-home.mjs"* ]]
  done

  run env COMPOSITION_CASE=order_insensitive "$fixture_root/scripts/evals/run.sh" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"--plugin-mode no-plugins"* ]]
  [[ "$output" == *"--plugin-mode targeted-plugins"* ]]

  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  for fixture in \
    "empty|providerCompositions must contain at least one provider" \
    "missing_composition_label|provider composition labels do not match configured providers: missing: codex-gpt-5.6-terra-no-plugins" \
    "extra_composition_label|provider composition labels do not match configured providers: extra: codex-gpt-5.6-terra-no-plugins"; do
    composition_case="${fixture%%|*}"
    expected="${fixture#*|}"
    grader_home="$fixture_root/grader-home-$composition_case"
    mkdir -p "$grader_home"
    printf 'ai-plugins Codex eval home\n' >"$grader_home/.ai-plugins-eval-home"
    printf 'preserve me\n' >"$grader_home/sentinel"

    run env \
      COMPOSITION_CASE="$composition_case" \
      OPENAI_API_KEY=fixture \
      PROMPTFOO_BIN=/bin/true \
      CODEX_EVAL_HOME="$grader_home" \
      CODEX_EVAL_HOME_FULL_MARKETPLACE="$grader_home" \
      "$fixture_root/scripts/evals/run.sh"

    [ "$status" -ne 0 ]
    [[ "$output" == *"$expected"* ]]
    [ -f "$grader_home/sentinel" ]
  done

  rm -rf "$fixture_root"
}

@test "package manifest pins promptfoo and coding harness provider SDKs" {
  run node - "$ROOT/package.json" "$ROOT/package-lock.json" <<'NODE'
const fs = require('fs');

const pkg = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const lock = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
const deps = pkg.devDependencies || {};
const expected = {
  promptfoo: '0.121.18',
  '@openai/codex-sdk': '0.144.5',
  '@anthropic-ai/claude-agent-sdk': '0.3.201',
};

for (const [name, version] of Object.entries(expected)) {
  if (deps[name] !== version) {
    throw new Error(`${name} should be pinned to ${version}, got ${deps[name] || 'missing'}`);
  }
}

if (pkg.overrides?.['@openai/codex-sdk'] !== expected['@openai/codex-sdk']) {
  throw new Error('Promptfoo must be forced onto the GPT-5.6-capable Codex SDK');
}

const resolvedSdkVersions = [...new Set(
  Object.entries(lock.packages || {})
    .filter(([entry]) => entry.endsWith('node_modules/@openai/codex-sdk'))
    .map(([, metadata]) => metadata.version),
)];
if (JSON.stringify(resolvedSdkVersions) !== JSON.stringify(['0.144.5'])) {
  throw new Error(`stale Codex SDK copies remain in package-lock: ${resolvedSdkVersions.join(', ')}`);
}
NODE

  [ "$status" -eq 0 ]
}
