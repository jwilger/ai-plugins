#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  CHECKER="$ROOT/scripts/evals/check-model-routing-campaign.mjs"
  CAMPAIGN="$ROOT/evals/benchmarks/model-routing/campaign.json"
  TMPROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "model-routing campaign declares the complete approved measurement contract" {
  run node "$CHECKER" "$CAMPAIGN"
  [ "$status" -eq 0 ]
  [[ "$output" == *"model-routing campaign contract valid"* ]]
}

@test "model-routing campaign rejects a quality gate without the paired confidence bound" {
  broken="$TMPROOT/campaign.json"
  jq 'del(.promotion.quality.candidate_only_failure_upper_bound)' "$CAMPAIGN" >"$broken"

  run node "$CHECKER" "$broken"
  [ "$status" -eq 2 ]
  [[ "$output" == *"candidate_only_failure_upper_bound"* ]]
}

@test "model-routing campaign rejects incomplete task and judge families" {
  broken="$TMPROOT/campaign.json"
  jq 'del(.task_families[0]) | del(.judge.rubric_families[0])' "$CAMPAIGN" >"$broken"

  run node "$CHECKER" "$broken"
  [ "$status" -eq 2 ]
  [[ "$output" == *"task_families"* ]]
}

@test "model-routing campaign rejects unsupported or incomplete model-effort coverage" {
  broken="$TMPROOT/campaign.json"
  jq '.candidates.models = ["gpt-5.6-luna"] | .candidates.efforts = ["medium"]' "$CAMPAIGN" >"$broken"

  run node "$CHECKER" "$broken"
  [ "$status" -eq 2 ]
  [[ "$output" == *"candidates"* ]]
}

@test "model-routing campaign keeps workflow and benchmark judge usage separate" {
  broken="$TMPROOT/campaign.json"
  jq '.cost.accounting.benchmark_judges = "included"' "$CAMPAIGN" >"$broken"

  run node "$CHECKER" "$broken"
  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark_judges"* ]]
}
