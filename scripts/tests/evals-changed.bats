#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p \
    "$TMPROOT/scripts/evals" \
    "$TMPROOT/evals/fixtures/behavior/development-discipline" \
    "$TMPROOT/evals/fixtures/behavior/development-system"
  cp "$ROOT/scripts/evals/run-changed.sh" "$TMPROOT/scripts/evals/run-changed.sh"
  cp "$ROOT/evals/fixtures/behavior/development-discipline/cases.json" \
    "$TMPROOT/evals/fixtures/behavior/development-discipline/cases.json"
  cp "$ROOT/evals/fixtures/behavior/development-system/cases.json" \
    "$TMPROOT/evals/fixtures/behavior/development-system/cases.json"
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

@test "changed eval planner skips test-only changes" {
  mkdir -p "$TMPROOT/scripts/tests"
  echo test >"$TMPROOT/scripts/tests/example.bats"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$BASE" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"no provider evals selected"* ]]
}

@test "changed eval planner ignores retired Pi runtime and guard paths" {
  mkdir -p \
    "$TMPROOT/scripts/evals" \
    "$TMPROOT/plugins/development-system/extensions/development-system/core"
  echo retired >"$TMPROOT/scripts/evals/pi-provider.mjs"
  echo retired >"$TMPROOT/plugins/development-system/extensions/development-system/core/guards.ts"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$BASE" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"no provider evals selected"* ]]
  [[ "$output" != *"Pi "* ]]
}

@test "changed eval planner ignores retired Pi autonomous goal paths" {
  mkdir -p "$TMPROOT/plugins/development-system/extensions/development-system/core"
  echo goal >"$TMPROOT/plugins/development-system/extensions/development-system/core/goal.ts"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$BASE" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"no provider evals selected"* ]]
  [[ "$output" != *"Pi "* ]]
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

@test "changed eval planner maps setup skill changes to development-system behavior cases" {
  mkdir -p "$TMPROOT/plugins/development-system/skills/setup"
  echo skill >"$TMPROOT/plugins/development-system/skills/setup/SKILL.md"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -qm "test: add skill"
  local skill_base
  skill_base="$(git -C "$TMPROOT" rev-parse HEAD)"
  echo changed >>"$TMPROOT/plugins/development-system/skills/setup/SKILL.md"
  commit_change

  run "$TMPROOT/scripts/evals/run-changed.sh" --base "$skill_base" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"shared skills [setup]"* ]]
  [[ "$output" == *"cases [2]"* ]]
}

@test "changed eval planner maps delivery skill changes across behavior fixture files" {
  mkdir -p "$TMPROOT/plugins/development-system/skills/delivery"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "%s\\n" "$EVAL_CASE_FILTER" >"$EVAL_FILTER_OUT"' \
    >"$TMPROOT/scripts/evals/run.sh"
  chmod +x "$TMPROOT/scripts/evals/run.sh"
  echo skill >"$TMPROOT/plugins/development-system/skills/delivery/SKILL.md"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -qm "test: add skill"
  local skill_base filter_file
  skill_base="$(git -C "$TMPROOT" rev-parse HEAD)"
  filter_file="$TMPROOT/case-filter.txt"
  echo changed >>"$TMPROOT/plugins/development-system/skills/delivery/SKILL.md"
  commit_change

  run env EVAL_FILTER_OUT="$filter_file" \
    "$TMPROOT/scripts/evals/run-changed.sh" --base "$skill_base"

  [ "$status" -eq 0 ]
  [[ "$(<"$filter_file")" == *"babysit-pr-natural-trigger"* ]]
  [[ "$(<"$filter_file")" == *"development-discipline-delivery-direct-to-trunk"* ]]
}

@test "changed eval planner selects matching development-system cases with stable de-duplicated IDs" {
  rm -rf "$TMPROOT/evals/fixtures/behavior"
  mkdir -p \
    "$TMPROOT/evals/fixtures/behavior/alpha" \
    "$TMPROOT/evals/fixtures/behavior/zeta" \
    "$TMPROOT/plugins/development-system/skills/delivery"
  printf '%s\n' \
    '[{"case_id":"z-case","plugins":["development-system"],"skills":["delivery"]},{"case_id":"duplicate-case","plugins":["development-system"],"skills":["delivery"]}]' \
    >"$TMPROOT/evals/fixtures/behavior/zeta/cases.json"
  printf '%s\n' \
    '[{"case_id":"a-case","plugins":["development-system"],"skills":["delivery"]},{"case_id":"duplicate-case","plugins":["development-system"],"skills":["delivery"]},{"case_id":"unrelated-case","plugins":["another-plugin"],"skills":["delivery"]}]' \
    >"$TMPROOT/evals/fixtures/behavior/alpha/cases.json"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "%s\\n" "$EVAL_CASE_FILTER" >"$EVAL_FILTER_OUT"' \
    >"$TMPROOT/scripts/evals/run.sh"
  chmod +x "$TMPROOT/scripts/evals/run.sh"
  echo skill >"$TMPROOT/plugins/development-system/skills/delivery/SKILL.md"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -qm "test: add behavior fixtures"
  local skill_base filter_file
  skill_base="$(git -C "$TMPROOT" rev-parse HEAD)"
  filter_file="$TMPROOT/case-filter.txt"
  echo changed >>"$TMPROOT/plugins/development-system/skills/delivery/SKILL.md"
  commit_change

  run env EVAL_FILTER_OUT="$filter_file" \
    "$TMPROOT/scripts/evals/run-changed.sh" --base "$skill_base"

  [ "$status" -eq 0 ]
  [[ "$output" == *"cases [3]"* ]]
  [ "$(<"$filter_file")" = '^(a-case|duplicate-case|z-case)$' ]
}
