#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run-code-quality-benchmark.sh"
  TEMP_ROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TEMP_ROOT"
}

@test "code-quality benchmark dry-run plans an isolated three-mode Rust feature slice without writing" {
  work_root="$TEMP_ROOT/workspaces"
  home_root="$TEMP_ROOT/homes"
  out_root="$TEMP_ROOT/out"

  run env \
    CODE_QUALITY_WORK_ROOT="$work_root" \
    CODE_QUALITY_HOME_ROOT="$home_root" \
    CODE_QUALITY_OUT_ROOT="$out_root" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 0 ]
  [[ "$output" == *"rust-cli-feature/sample-1/no-plugins"* ]]
  [[ "$output" == *"rust-cli-feature/sample-1/targeted-plugins"* ]]
  [[ "$output" == *"rust-cli-feature/sample-1/full-marketplace"* ]]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 3 ]
  [[ "$output" == *"$home_root/no-plugins --plugin-mode no-plugins"* ]]
  [[ "$output" == *"$home_root/targeted-plugins --plugin-mode targeted-plugins --plugins advisor\,development-discipline\,engineering-standards"* ]]
  [[ "$output" == *"$home_root/full-marketplace --plugin-mode full-marketplace"* ]]
  [[ "$output" == *"openai-codex-sdk-no-plugins"* ]]
  [[ "$output" == *"openai-codex-sdk-targeted-plugins"* ]]
  [[ "$output" == *"openai-codex-sdk-full-marketplace"* ]]
  [[ "$output" == *"execution EVAL_CASE_FILTER=rust-cli-feature EVAL_SAMPLES=1"* ]]
  [[ "$output" == *"--filter-pattern rust-cli-feature"* ]]
  [[ "$output" == *"promotion gates disabled: diagnostic noncanonical run"* ]]
  [[ "$output" != *"gate targeted-overall"* ]]
  [[ "$output" == *"$out_root/results.json"* ]]
  [[ "$output" == *"check-code-quality-benchmark.mjs"* ]]
  [ ! -e "$work_root" ]
  [ ! -e "$home_root" ]
  [ ! -e "$out_root" ]
}

@test "code-quality benchmark rejects overlapping workspace and Codex-home roots before planning" {
  work_root="$TEMP_ROOT/workspaces"
  home_root="$work_root/rust-cli-feature/sample-1"

  run env \
    CODE_QUALITY_WORK_ROOT="$work_root" \
    CODE_QUALITY_HOME_ROOT="$home_root" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark paths overlap"* ]]
  [[ "$output" != *"prepare-codex-home.mjs"* ]]
  [ ! -e "$work_root" ]
}

@test "code-quality benchmark recognizes root and delimiter characters in overlapping paths" {
  run env \
    CODE_QUALITY_WORK_ROOT=/ \
    CODE_QUALITY_HOME_ROOT=/rust-cli-feature/sample-1 \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/root-out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark paths overlap"* ]]

  work_root="$TEMP_ROOT/work|spaces"
  run env \
    CODE_QUALITY_WORK_ROOT="$work_root" \
    CODE_QUALITY_HOME_ROOT="$work_root/rust-cli-feature/sample-1" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/delimiter-out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark paths overlap"* ]]
}

@test "code-quality benchmark default dry-run predeclares three task types by three modes by three samples" {
  run env \
    CODE_QUALITY_WORK_ROOT="$TEMP_ROOT/workspaces" \
    CODE_QUALITY_HOME_ROOT="$TEMP_ROOT/homes" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/out" \
    "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^workspace ')" -eq 27 ]
  [[ "$output" == *"rust-cli-feature/sample-3/full-marketplace"* ]]
  [[ "$output" == *"stock-service-bugfix/sample-3/full-marketplace"* ]]
  [[ "$output" == *"stock-service-refactor/sample-3/full-marketplace"* ]]
  [[ "$output" == *"metric pass@3 capability"* ]]
  [[ "$output" == *"metric pass^3 reliability"* ]]
  [[ "$output" == *"gate targeted-overall 8/9"* ]]
  [[ "$output" == *"gate full-overall 7/9"* ]]
  [[ "$output" == *"gate targeted-lift 2/9"* ]]
  [[ "$output" == *"gate targeted-per-case-no-regression >=0"* ]]
}
