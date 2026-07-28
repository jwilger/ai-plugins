#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p "$TMPROOT/scripts/evals" "$TMPROOT/evals/fixtures/agentic-systems-engineering"
  cp "$ROOT/scripts/evals/run-changed.sh" "$TMPROOT/scripts/evals/run-changed.sh"
  cp "$ROOT/evals/fixtures/agentic-systems-engineering/cases.json" \
    "$TMPROOT/evals/fixtures/agentic-systems-engineering/cases.json"
  git -C "$TMPROOT" init -q --initial-branch=main
  git -C "$TMPROOT" config user.name "Eval Test"
  git -C "$TMPROOT" config user.email "eval@example.invalid"
  echo baseline >"$TMPROOT/README.md"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -qm "test: baseline"
  BASE="$(git -C "$TMPROOT" rev-parse HEAD)"
}

teardown() {
  rm -rf "$TMPROOT"
}

commit_change() {
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -qm "test: change"
}

@test "changed eval planner skips documentation-only changes" {
  echo docs >>"$TMPROOT/README.md"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$BASE" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"no provider evals selected"* ]]
}

@test "changed eval planner selects only Pi package canary and guard outcomes" {
  mkdir -p "$TMPROOT/plugins/development-system/extensions/development-system/core"
  echo guard >"$TMPROOT/plugins/development-system/extensions/development-system/core/guards.ts"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$BASE" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"Pi package/runtime"* ]]
  [[ "$output" == *"Pi guard/runtime"* ]]
  [[ "$output" != *"all supported harnesses"* ]]
}

@test "changed eval planner selects only autonomous goal evidence for goal code" {
  mkdir -p "$TMPROOT/plugins/development-system/extensions/development-system/core"
  echo goal >"$TMPROOT/plugins/development-system/extensions/development-system/core/goal.ts"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$BASE" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"Pi package/runtime"* ]]
  [[ "$output" == *"Pi autonomous goal"* ]]
  [[ "$output" != *"Pi guard/runtime"* ]]
}

@test "changed eval planner maps shared skill changes to affected behavior cases" {
  mkdir -p "$TMPROOT/plugins/development-system/skills/eval-case-reporting"
  echo skill >"$TMPROOT/plugins/development-system/skills/eval-case-reporting/SKILL.md"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -qm "test: add skill"
  local skill_base
  skill_base="$(git -C "$TMPROOT" rev-parse HEAD)"
  echo changed >>"$TMPROOT/plugins/development-system/skills/eval-case-reporting/SKILL.md"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$skill_base" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"shared skills [eval-case-reporting]"* ]]
  [[ "$output" == *"cases [1]"* ]]
  [[ "$output" == *"all supported harnesses, one sample"* ]]
}
