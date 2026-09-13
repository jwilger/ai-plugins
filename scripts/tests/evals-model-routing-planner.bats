#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PLANNER="$ROOT/scripts/evals/plan-model-routing-campaign.mjs"
  CAMPAIGN="$ROOT/evals/benchmarks/model-routing/campaign.json"
  TMPROOT="$(mktemp -d)"
  PLAN="$TMPROOT/plan.json"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "screening plan expands the complete fixed model effort matrix" {
  run node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  [ "$status" -eq 0 ]

  [ "$(jq -r '.schema_version' "$PLAN")" = "1" ]
  [ "$(jq -r '.phase' "$PLAN")" = "screening" ]
  [ "$(jq '.jobs | length' "$PLAN")" -eq 992 ]
  [ "$(jq '[.jobs[].task_family] | unique | length' "$PLAN")" -eq 6 ]
  [ "$(jq '[.jobs[] | select(.task_family == "mechanical-assistance") | .effort] | unique | sort == ["high", "low", "max", "medium", "none", "xhigh"]' "$PLAN")" = "true" ]
  [ "$(jq '[.jobs[] | select(.task_family != "mechanical-assistance") | .effort] | unique | sort == ["high", "low", "max", "medium", "xhigh"]' "$PLAN")" = "true" ]
  [ "$(jq '[.jobs[].status] | unique == ["pending"]' "$PLAN")" = "true" ]
  [ "$(jq '[.jobs[].job_id] | length == (unique | length)' "$PLAN")" = "true" ]
}

@test "planner output is byte-stable for the same campaign" {
  second="$TMPROOT/second.json"
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$second"

  run cmp "$PLAN" "$second"
  [ "$status" -eq 0 ]
}

@test "resume preserves completed outcomes and does not duplicate jobs" {
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  completed_id="$(jq -r '.jobs[0].job_id' "$PLAN")"
  jq --arg id "$completed_id" '(.jobs[] | select(.job_id == $id)) += {
    status: "success",
    attempt_count: 1,
    result_ref: "checkpoints/example.json"
  }' "$PLAN" >"$TMPROOT/resume.json"
  mv "$TMPROOT/resume.json" "$PLAN"

  run node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN" --resume
  [ "$status" -eq 0 ]
  [ "$(jq --arg id "$completed_id" -r '.jobs[] | select(.job_id == $id) | .status' "$PLAN")" = "success" ]
  [ "$(jq --arg id "$completed_id" -r '.jobs[] | select(.job_id == $id) | .result_ref' "$PLAN")" = "checkpoints/example.json" ]
  [ "$(jq '.jobs | length' "$PLAN")" -eq 992 ]
}

@test "resume fails closed when the campaign identity changes" {
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  jq '.campaign_id = "different-campaign"' "$PLAN" >"$TMPROOT/resume.json"
  mv "$TMPROOT/resume.json" "$PLAN"

  run node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN" --resume
  [ "$status" -eq 2 ]
  [[ "$output" == *"campaign identity"* ]]
}

@test "resume rejects a job whose fields no longer match its stable identity" {
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  jq '.jobs[0].model = "gpt-6-astra"' "$PLAN" >"$TMPROOT/resume.json"
  mv "$TMPROOT/resume.json" "$PLAN"

  run node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN" --resume
  [ "$status" -eq 2 ]
  [[ "$output" == *"job identity fields"* ]]
}

@test "planner rejects unsupported phases instead of silently narrowing scope" {
  run node "$PLANNER" "$CAMPAIGN" --phase cheap-only --output "$PLAN"
  [ "$status" -eq 2 ]
  [[ "$output" == *"unsupported phase"* ]]
}
