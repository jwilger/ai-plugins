#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PLANNER="$ROOT/scripts/evals/plan-model-routing-campaign.mjs"
  RUNNER="$ROOT/scripts/evals/run-model-routing-job.mjs"
  CODEX_JUDGE_PROVIDER="$ROOT/evals/benchmarks/model-routing/codex-judge-provider.mjs"
  CODEX_SUBJECT_PROVIDER="$ROOT/evals/benchmarks/model-routing/codex-subject-provider.mjs"
  CAMPAIGN="$ROOT/evals/benchmarks/model-routing/campaign.json"
  CASES="$ROOT/evals/benchmarks/model-routing/cases.json"
  FIXTURE_SPECS="$ROOT/evals/benchmarks/model-routing/fixture-specs.json"
  MATERIALIZER="$ROOT/scripts/evals/materialize-model-routing-fixtures.mjs"
  VERIFIER="$ROOT/scripts/evals/verify-model-routing-result.mjs"
  TMPROOT="$(mktemp -d)"
  PLAN="$TMPROOT/plan.json"
}

@test "Codex subject adapter binds a prepared fixture workspace and execution surface" {
  run node --input-type=module - "$CODEX_SUBJECT_PROVIDER" <<'EOF'
import { pathToFileURL } from "node:url";
const { createCodexSubjectProviderFunction } = await import(pathToFileURL(process.argv[2]));
let observed;
const inner = {
  async callApi(prompt) {
    if (prompt !== "implement fix") throw new Error("unexpected prompt");
    return {
      output: "done",
      tokenUsage: { prompt: 13, cached: 1, completion: 7, total: 20 },
      raw: JSON.stringify({ notifications: [] })
    };
  },
  async cleanup() {}
};
const run = createCodexSubjectProviderFunction({
  providerLoader: async (id, options) => {
    observed = { id, options };
    return inner;
  },
  prepareWorkspace: () => "/tmp/fixed-workspace",
  environment: {
    MODEL_ROUTING_CODEX_HOME: "/tmp/codex-home",
    MODEL_ROUTING_TOOL_PATH: "/nix/store/example-tools/bin"
  },
  now: (() => { const values = [10, 55]; return () => values.shift(); })()
});
const result = await run({
  job_id: "implementation/case-001/gpt-5.6-terra/high",
  prompt: "implement fix",
  fixture: "implementation-001",
  model: "gpt-5.6-terra",
  effort: "high",
  execution_surface: "workspace-write"
});
console.log(JSON.stringify({ result, observed }));
EOF
  [ "$status" -eq 0 ]
  [ "$(jq -r '.observed.id' <<<"$output")" = "openai:codex-app-server" ]
  [ "$(jq -r '.observed.options.options.config.model' <<<"$output")" = "gpt-5.6-terra" ]
  [ "$(jq -r '.observed.options.options.config.model_reasoning_effort' <<<"$output")" = "high" ]
  [ "$(jq -r '.observed.options.options.config.working_dir' <<<"$output")" = "/tmp/fixed-workspace" ]
  [ "$(jq -r '.observed.options.options.config.sandbox_mode' <<<"$output")" = "workspace-write" ]
  [ "$(jq -r '.observed.options.options.config.network_access_enabled' <<<"$output")" = "false" ]
  [ "$(jq -r '.observed.options.options.config.cli_config.features.plugins' <<<"$output")" = "true" ]
  [ "$(jq -r '.result.usage.input_tokens' <<<"$output")" -eq 13 ]
  [ "$(jq -r '.result.elapsed_ms' <<<"$output")" -eq 45 ]
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

  run node "$RUNNER" --plan "$PLAN" --job-id "$job_id" --cases "$CASES" --fixture-specs "$FIXTURE_SPECS" --results "$result_root" --provider "$provider"
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
  node "$RUNNER" --plan "$PLAN" --job-id "$job_id" --cases "$CASES" --fixture-specs "$FIXTURE_SPECS" --results "$result_root" --provider "$provider"
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

  run node "$RUNNER" --plan "$PLAN" --job-id "$job_id" --cases "$CASES" --fixture-specs "$FIXTURE_SPECS" --results "$result_root" --provider "$provider"
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
  [[ "$(jq -r '.case_catalog_sha256' "$PLAN")" =~ ^[0-9a-f]{64}$ ]]
  [[ "$(jq -r '.fixture_specs_sha256' "$PLAN")" =~ ^[0-9a-f]{64}$ ]]
  [ "$(jq '.jobs | length' "$PLAN")" -eq 992 ]
  [ "$(jq '[.jobs[].task_family] | unique | length' "$PLAN")" -eq 6 ]
  [ "$(jq '[.jobs[] | select(.task_family == "mechanical-assistance") | .effort] | unique | sort == ["high", "low", "max", "medium", "none", "xhigh"]' "$PLAN")" = "true" ]
  [ "$(jq '[.jobs[] | select(.task_family != "mechanical-assistance") | .effort] | unique | sort == ["high", "low", "max", "medium", "xhigh"]' "$PLAN")" = "true" ]
  [ "$(jq '[.jobs[].status] | unique == ["pending"]' "$PLAN")" = "true" ]
  [ "$(jq '[.jobs[].job_id] | length == (unique | length)' "$PLAN")" = "true" ]
}

@test "fixed workspace fixtures materialize byte-stably and verify mechanically" {
  first="$TMPROOT/fixtures-first"
  second="$TMPROOT/fixtures-second"
  run node "$MATERIALIZER" --specs "$FIXTURE_SPECS" --output "$first"
  [ "$status" -eq 0 ]
  node "$MATERIALIZER" --specs "$FIXTURE_SPECS" --output "$second"
  run diff -ru "$first" "$second"
  [ "$status" -eq 0 ]
  [ -f "$first/mechanical-001/config.txt" ]

  cp -R "$first/mechanical-001" "$TMPROOT/completed"
  sed -i 's/OLD_ROUTE/NEW_ROUTE/' "$TMPROOT/completed/config.txt"
  run node "$VERIFIER" --specs "$FIXTURE_SPECS" --fixture mechanical-001 --workspace "$TMPROOT/completed" --output "NEW_ROUTE"
  [ "$status" -eq 0 ]
  [ "$(jq -r '.status' <<<"$output")" = "pass" ]
  [ "$(jq -r '.method' <<<"$output")" = "exact-workspace" ]
}

@test "every mechanical fixture accepts only its exact expected workspace" {
  fixtures="$TMPROOT/fixtures"
  node "$MATERIALIZER" --specs "$FIXTURE_SPECS" --output "$fixtures"
  while IFS= read -r fixture; do
    workspace="$TMPROOT/$fixture-completed"
    mkdir "$workspace"
    node --input-type=module - "$FIXTURE_SPECS" "$fixture" "$workspace" <<'EOF'
import fs from "node:fs";
import path from "node:path";
const document = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const fixture = document.fixtures.find((item) => item.fixture_id === process.argv[3]);
for (const [file, contents] of Object.entries(fixture.verification.expected_files)) {
  const destination = path.join(process.argv[4], file);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, contents);
}
EOF
    run node "$VERIFIER" --specs "$FIXTURE_SPECS" --fixture "$fixture" --workspace "$workspace" --output ignored
    [ "$status" -eq 0 ]
    [ "$(jq -r '.status' <<<"$output")" = "pass" ]
  done < <(jq -r '.fixtures[].fixture_id' "$FIXTURE_SPECS")
}

@test "resume rejects tampered fixed-input hashes" {
  node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN"
  jq '.fixture_specs_sha256 = ("0" * 64)' "$PLAN" >"$TMPROOT/tampered.json"
  mv "$TMPROOT/tampered.json" "$PLAN"

  run node "$PLANNER" "$CAMPAIGN" --phase screening --output "$PLAN" --resume
  [ "$status" -eq 2 ]
  [[ "$output" == *"campaign identity"* ]]
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
