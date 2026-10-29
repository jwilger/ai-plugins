#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "workspace preparation refuses protected overlap before creating directories" {
  temp_root="$(mktemp -d)"
  for relationship in same ancestor descendant; do
    case "$relationship" in
      same) workspace="$temp_root/protected" ;;
      ancestor) workspace="$temp_root" ;;
      descendant) workspace="$temp_root/protected/nested" ;;
    esac
    run node "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
      "$workspace" --forbid-overlap "$temp_root/protected"
    [ "$status" -eq 2 ]
    [[ "$output" == *"workspace overlaps protected path"* ]]
    [ ! -e "$temp_root/protected" ]
  done
  rm -rf "$temp_root"
}

@test "workspace preparation preserves an unowned nonempty directory" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  printf 'keep me\n' >"$workspace/user-file"
  run node "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" "$workspace"
  [ "$status" -eq 2 ]
  [[ "$output" == *"refusing to replace unowned"* ]]
  grep -q 'keep me' "$workspace/user-file"
  rm -rf "$temp_root"
}

@test "workspace preparation clears stale instructions only in a marker-owned directory" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' >"$workspace/.ai-plugins-gpt56-workspace"
  printf 'stale instructions\n' >"$workspace/AGENTS.md"
  run node "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" "$workspace"
  [ "$status" -eq 0 ]
  [ ! -e "$workspace/AGENTS.md" ]
  [ -f "$workspace/.ai-plugins-gpt56-workspace" ]
  rm -rf "$temp_root"
}

@test "Codex home preparation initializes an empty unmarked directory in place" {
  temp_root="$(mktemp -d)"
  eval_home="$temp_root/codex-home"
  mkdir "$eval_home"
  chmod 0711 "$eval_home"
  setfacl -m d:u::rwx,d:g::r-x,d:o::--- "$eval_home"
  original_inode="$(stat -c %i "$eval_home")"
  original_acl="$(getfacl -cp "$eval_home")"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  [ "$(stat -c %i "$eval_home")" = "$original_inode" ]
  [ "$(stat -c %a "$eval_home")" = "711" ]
  [ "$(getfacl -cp "$eval_home")" = "$original_acl" ]
  [ -f "$eval_home/.ai-plugins-eval-home" ]

  rm -rf "$temp_root"
}

@test "GPT-5.6 workspace preparation initializes an empty unmarked directory in place" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  chmod 0711 "$workspace"
  setfacl -m d:u::rwx,d:g::r-x,d:o::--- "$workspace"
  original_inode="$(stat -c %i "$workspace")"
  original_acl="$(getfacl -cp "$workspace")"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace"

  [ "$status" -eq 0 ]
  [ "$(stat -c %i "$workspace")" = "$original_inode" ]
  [ "$(stat -c %a "$workspace")" = "711" ]
  [ "$(getfacl -cp "$workspace")" = "$original_acl" ]
  [ -f "$workspace/.ai-plugins-gpt56-workspace" ]

  rm -rf "$temp_root"
}

@test "GPT-5.6 workspace check rejects an ancestor AGENTS.md without creating the workspace" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/project/nested/workspace"
  mkdir -p "$temp_root/project"
  printf '%s\n' '# Host instructions' >"$temp_root/project/AGENTS.md"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace" \
    --check

  actual_status="$status"
  actual_output="$output"
  workspace_was_created=0
  [ ! -e "$workspace" ] || workspace_was_created=1
  rm -rf "$temp_root"

  [ "$actual_status" -eq 2 ]
  [[ "$actual_output" == *"AGENTS.md"* ]]
  [ "$workspace_was_created" -eq 0 ]
}

@test "GPT-5.6 workspace preparation rejects an ancestor AGENTS.md without creating the workspace" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/project/nested/workspace"
  mkdir -p "$temp_root/project"
  printf '%s\n' '# Host instructions' >"$temp_root/project/AGENTS.md"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace"

  actual_status="$status"
  workspace_was_created=0
  [ ! -e "$workspace" ] || workspace_was_created=1
  rm -rf "$temp_root"

  [ "$actual_status" -eq 2 ]
  [ "$workspace_was_created" -eq 0 ]
}

@test "GPT-5.6 workspace check rejects a Git checkout without creating the workspace" {
  temp_root="$(mktemp -d)"
  checkout="$temp_root/checkout"
  workspace="$checkout/nested/workspace"
  git init -q "$checkout"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace" \
    --check

  actual_status="$status"
  actual_output="$output"
  workspace_was_created=0
  [ ! -e "$workspace" ] || workspace_was_created=1
  rm -rf "$temp_root"

  [ "$actual_status" -eq 2 ]
  [[ "$actual_output" == *"Git"* || "$actual_output" == *"git"* ]]
  [ "$workspace_was_created" -eq 0 ]
}

@test "GPT-5.6 workspace preparation tolerates an invalid enclosing .git entry" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  printf 'gitdir: /does/not/exist\n' >"$temp_root/.git"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace"

  actual_status="$status"
  marker_contents=""
  if [ -f "$workspace/.ai-plugins-gpt56-workspace" ]; then
    marker_contents="$(cat "$workspace/.ai-plugins-gpt56-workspace")"
  fi
  rm -rf "$temp_root"

  [ "$actual_status" -eq 0 ]
  [ "$marker_contents" = "ai-plugins GPT-5.6 benchmark workspace" ]
}

@test "Codex home preparation removes stale eval state before rebuilding" {
  temp_root="$(mktemp -d)"
  eval_home="$temp_root/codex-home"
  mkdir -p \
    "$eval_home/sessions" \
    "$eval_home/plugins/cache/ai-plugins/stale-plugin/0.0.0" \
    "$eval_home/skills/stale-system-skill" \
    "$eval_home/apps/stale-app"
  printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
  printf 'stale\n' >"$eval_home/sessions/stale-session.jsonl"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  [ -f "$eval_home/config.toml" ]
  [ ! -e "$eval_home/sessions" ]
  [ ! -e "$eval_home/plugins" ]
  [ ! -e "$eval_home/skills" ]
  [ ! -e "$eval_home/apps" ]

  rm -rf "$temp_root"
}

@test "Codex home preparation preserves unowned and auth-overlapping paths" {
  temp_root="$(mktemp -d)"
  unowned_home="$temp_root/unowned"
  mkdir -p "$unowned_home"
  printf 'keep me\n' >"$unowned_home/user-file"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$unowned_home" \
    --plugin-mode no-plugins

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to replace unowned Codex eval home"* ]]
  [ -f "$unowned_home/user-file" ]

  auth_parent="$temp_root/auth-parent"
  auth_home="$auth_parent/auth-source"
  mkdir -p "$auth_home"
  printf 'ai-plugins Codex eval home\n' >"$auth_parent/.ai-plugins-eval-home"
  printf '{"token":"keep"}\n' >"$auth_home/auth.json"

  run env CODEX_EVAL_AUTH_HOME="$auth_home" node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$auth_parent" \
    --plugin-mode no-plugins

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing Codex eval home path that overlaps the auth source"* ]]
  grep -q 'keep' "$auth_home/auth.json"

  rm -rf "$temp_root"
}

@test "Codex home preparation refuses an unowned home with marketplace config" {
  temp_root="$(mktemp -d)"
  unowned_home="$temp_root/alternate-codex-home"
  mkdir -p "$unowned_home"
  printf '[marketplaces.ai-plugins]\nsource = "fixture"\n' >"$unowned_home/config.toml"
  printf 'keep me\n' >"$unowned_home/unrelated-session"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$unowned_home" \
    --plugin-mode no-plugins

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to replace unowned Codex eval home"* ]]
  grep -q 'keep me' "$unowned_home/unrelated-session"

  rm -rf "$temp_root"
}
