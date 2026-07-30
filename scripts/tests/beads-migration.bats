#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PROJECT="$(mktemp -d)"
  git -C "$PROJECT" init -q --initial-branch=main
  git -C "$PROJECT" config user.name "Migration Test"
  git -C "$PROJECT" config user.email "migration@example.invalid"
  cat >"$PROJECT/.development-system.toml" <<'TOML'
schema_version = 1
[delivery]
mode = "direct-to-trunk"
trunk_branch = "main"
[features]
worktrees = true
tiber = true
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[tiber]
max_queued = 5
TOML
  printf 'fixture\n' >"$PROJECT/README.md"
  git -C "$PROJECT" add .
  git -C "$PROJECT" commit -qm "test: initialize migration fixture"

  git -C "$PROJECT" checkout -q --orphan tasks
  git -C "$PROJECT" rm -q -rf .
  mkdir -p "$PROJECT/backlog" "$PROJECT/in-progress" "$PROJECT/done" "$PROJECT/abandoned"
  cat >"$PROJECT/in-progress/20260730-aaaa-migrate-work.md" <<'TASK'
---
title: Migrate work
blocked_by: []
blocks: [20260730-bbbb-follow-up]
tags: [feature, migration]
pr_mr_url:
pr_mr_status:
---

## Summary

Move task state into Beads.

## Context / Why

The legacy tracker is retired.

## Acceptance criteria

- [ ] History is retained

## Subtasks

## Notes / Log
TASK
  cat >"$PROJECT/backlog/20260730-bbbb-follow-up.md" <<'TASK'
---
title: Follow up
blocked_by: [20260730-aaaa-migrate-work]
blocks: []
tags: [task]
pr_mr_url:
pr_mr_status:
---

## Summary

Complete follow-up work.

## Context / Why

It depends on migration.

## Acceptance criteria

- [ ] Follow-up is ready later

## Subtasks

## Notes / Log
TASK
  printf '%s\n' 20260730-aaaa-migrate-work 20260730-bbbb-follow-up >"$PROJECT/order.md"
  git -C "$PROJECT" add .
  git -C "$PROJECT" commit -qm "test: create legacy task board"
  git -C "$PROJECT" checkout -q main
}

teardown() {
  rm -rf -- "$PROJECT"
}

@test "migration previews without mutation then imports history into Dolt in one source commit" {
  before_head="$(git -C "$PROJECT" rev-parse HEAD)"

  run "$ROOT/plugins/development-system/bin/development-system" \
    migrate-tiber-to-beads --project "$PROJECT" --prefix ai --dry-run

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e '.mode == "dry-run" and .issues == 2 and .migratesPolicy == true and .formulas >= 10' >/dev/null
  [ "$(git -C "$PROJECT" rev-parse HEAD)" = "$before_head" ]
  [ ! -e "$PROJECT/.beads" ]

  run "$ROOT/plugins/development-system/bin/development-system" \
    migrate-tiber-to-beads --project "$PROJECT" --prefix ai --apply --yes

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e '.mode == "applied" and .issues == 2 and .commit != null and .pushedDolt == false' >/dev/null
  [ "$(git -C "$PROJECT" rev-list --count main)" -eq 2 ]
  grep -Fq 'schema_version = 2' "$PROJECT/.development-system.toml"
  grep -Fq 'beads = true' "$PROJECT/.development-system.toml"
  [ -f "$PROJECT/.beads/formulas/behavior-slice.formula.toml" ]
  run bash -c 'cd "$1" && bd show ai-tiber-20260730-aaaa-migrate-work --json' _ "$PROJECT"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"source_system": "tiber"'* ]]
}
