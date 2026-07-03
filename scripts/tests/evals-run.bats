#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run.sh"
}

@test "eval runner prints help" {
  run "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage: scripts/evals/run.sh"* ]]
  [[ "$output" == *"results.junit.xml"* ]]
}

@test "eval runner dry-run uses local promptfoo config and repo-owned artifacts" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"promptfoo@"* ]]
  [[ "$output" == *"evals/promptfoo/agentic-systems-engineering.yaml"* ]]
  [[ "$output" == *"evals/out/results.json"* ]]
  [[ "$output" == *"evals/out/report.html"* ]]
  [[ "$output" == *"evals/out/results.junit.xml"* ]]
}
