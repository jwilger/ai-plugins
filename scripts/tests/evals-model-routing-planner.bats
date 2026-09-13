#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PLANNER="$ROOT/scripts/evals/plan-model-routing-campaign.mjs"
  RUNNER="$ROOT/scripts/evals/run-model-routing-job.mjs"
  CODEX_JUDGE_PROVIDER="$ROOT/evals/benchmarks/model-routing/codex-judge-provider.mjs"
  CAMPAIGN="$ROOT/evals/benchmarks/model-routing/campaign.json"
  CASES="$ROOT/evals/benchmarks/model-routing/cases.json"
  TMPROOT="$(mktemp -d)"
  PLAN="$TMPROOT/plan.json"
}

@test "Codex judge adapter pins model and effort while preserving isolated execution and usage" {
  run node --input-type=module - "$CODEX_JUDGE_PROVIDER" <<'EOF'
import { pathToFileURL } from "node:url";
const { createCodexJudgeProviderFunction } = await import(pathToFileURL(process.argv[2]));
let observed;
class FakeProvider {
  constructor(options) { observed = options; }
  async callApi(prompt) {
    if (prompt !== "fixed prompt") throw new Error("unexpected prompt");
    return {
      output: "answer",
      tokenUsage: { prompt: 17, cached: 4, completion: 5, total: 22 },
      raw: JSON.stringify({
        notifications: [{ method: "error", params: { willRetry: true } }]
      })
    };
  }
  async cleanup() {}
}
const run = createCodexJudgeProviderFunction({
  ProviderClass: FakeProvider,
  environment: {
    MODEL_ROUTING_CODEX_HOME: "/tmp/codex-home",
    MODEL_ROUTING_WORKSPACE: "/tmp/workspace"
  },
  now: (() => { const values = [100, 145]; return () => values.shift(); })()
});
const result = await run({
  prompt: "fixed prompt",
  model: "gpt-5.6-luna",
  effort: "max",
  isolation: { plugins: false, tools: false, workspace: false }
});
console.log(JSON.stringify({ result, observed }));
EOF
  [ "$status" -eq 0 ]
  [ "$(jq -r '.observed.config.model' <<<"$output")" = "gpt-5.6-luna" ]
  [ "$(jq -r '.observed.config.model_reasoning_effort' <<<"$output")" = "max" ]
  [ "$(jq -r '.observed.config.cli_config.features.plugins' <<<"$output")" = "false" ]
  [ "$(jq -r '.result.usage.cached_input_tokens' <<<"$output")" -eq 4 ]
  [ "$(jq -r '.result.attempts' <<<"$output")" -eq 2 ]
}

@test "runner resolves a fixed case and records a successful subject result" {
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  job_id="mechanical-assistance/case-001/gpt-5.6-luna/none"
  provider="$TMPROOT/provider.mjs"
  result_root="$TMPROOT/results"
  cat >"$provider" <<'EOF'
export default async function run(request) {
  if (request.prompt !== "Replace the exact token OLD_ROUTE with NEW_ROUTE in config.txt.") throw new Error("unexpected prompt");
  return {
    status: "success",
    output: "NEW_ROUTE",
    usage: { input_tokens: 11, cached_input_tokens: 2, output_tokens: 3 },
    elapsed_ms: 40,
    attempts: 1
  };
}
EOF

  run node "$RUNNER" --plan "$PLAN" --job-id "$job_id" --cases "$CASES" --results "$result_root" --provider "$provider"
  [ "$status" -eq 0 ]
  [ "$(jq --arg id "$job_id" -r '.jobs[] | select(.job_id == $id) | .status' "$PLAN")" = "success" ]
  result_ref="$(jq --arg id "$job_id" -r '.jobs[] | select(.job_id == $id) | .result_ref' "$PLAN")"
  [ -f "$result_root/$result_ref" ]
  [ "$(jq -r '.request.model' "$result_root/$result_ref")" = "gpt-5.6-luna" ]
  [ "$(jq -r '.request.effort' "$result_root/$result_ref")" = "none" ]
  [ "$(jq -r '.cost.subject.input_tokens' "$result_root/$result_ref")" -eq 11 ]
  [ "$(jq -r '.cost.judges' "$result_root/$result_ref")" = "null" ]
}

@test "runner resumes from a persisted result without repeating the provider call" {
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  job_id="mechanical-assistance/case-001/gpt-5.6-luna/none"
  provider="$TMPROOT/provider.mjs"
  result_root="$TMPROOT/results"
  cat >"$provider" <<'EOF'
export default async function run() {
  return {
    status: "success",
    output: "NEW_ROUTE",
    usage: { input_tokens: 11, cached_input_tokens: 2, output_tokens: 3 },
    elapsed_ms: 40,
    attempts: 1
  };
}
EOF
  node "$RUNNER" --plan "$PLAN" --job-id "$job_id" --cases "$CASES" --results "$result_root" --provider "$provider"
  jq --arg id "$job_id" '(.jobs[] | select(.job_id == $id)) += {
    status: "pending",
    attempt_count: 0,
    result_ref: null
  }' "$PLAN" >"$TMPROOT/interrupted.json"
  mv "$TMPROOT/interrupted.json" "$PLAN"
  cat >"$provider" <<'EOF'
export default async function run() {
  throw new Error("provider must not be called during recovery");
}
EOF

  run node "$RUNNER" --plan "$PLAN" --job-id "$job_id" --cases "$CASES" --results "$result_root" --provider "$provider"
  [ "$status" -eq 0 ]
  [ "$(jq --arg id "$job_id" -r '.jobs[] | select(.job_id == $id) | .status' "$PLAN")" = "success" ]
  [ "$(jq --arg id "$job_id" -r '.jobs[] | select(.job_id == $id) | .attempt_count' "$PLAN")" -eq 1 ]
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
