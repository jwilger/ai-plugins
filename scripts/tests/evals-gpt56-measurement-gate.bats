#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RESULTS="$(mktemp)"
}

teardown() {
  rm -f "$RESULTS"
}

run_checker() {
  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$RESULTS"
}

run_configured_checker() {
  local case_id="$1"
  local providers="$2"
  local samples="${3:-1}"
  local configured="${RESULTS}.configured"

  jq \
    --arg case_id "$case_id" \
    --argjson providers "$providers" \
    --argjson samples "$samples" \
    '.config.tests = [
      range(1; $samples + 1) as $sample |
      {
        providers: $providers,
        vars: {
          case_id: $case_id,
          min_pass_rate: 0,
          value_gate_mode: "measurement",
          benchmark_expected_provider_labels: $providers,
          benchmark_expected_samples: $samples,
          sample_index: $sample
        }
      }
    ]' \
    "$RESULTS" >"$configured"
  mv "$configured" "$RESULTS"
  run_checker
}

write_valid_paid_measurement_artifact() {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {"output": "Complete Sol answer"},
    "success": true,
    "latencyMs": 1250,
    "cost": 0.42,
    "tokenUsage": {
      "prompt": 120,
      "completion": 30,
      "total": 150,
      "cached": 20,
      "assertions": {
        "prompt": 80,
        "completion": 10,
        "total": 90,
        "cached": 15
      }
    },
    "gradingResult": {
      "pass": true,
      "score": 1,
      "reason": "Rubric satisfied",
      "componentResults": [{
        "pass": true,
        "score": 1,
        "reason": "Rubric satisfied"
      }]
    },
    "testCase": {"vars": {
      "case_id": "comparison-case", "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
      "benchmark_expected_samples": 1, "sample_index": 1
    }}
  }]}
}
JSON
}

mutate_paid_measurement_artifact() {
  local filter="$1"
  local mutated="${RESULTS}.mutated"

  jq "$filter" "$RESULTS" >"$mutated"
  mv "$mutated" "$RESULTS"
}

assert_paid_measurement_mutation_rejected() {
  local filter="$1"
  local expected="$2"

  write_valid_paid_measurement_artifact
  mutate_paid_measurement_artifact "$filter"
  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"$expected"* ]]
}

@test "measurement mode rejects a configless result artifact" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {"output": "Complete Sol answer"},
    "success": true,
    "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
    "testCase": {"vars": {
      "case_id": "comparison-case", "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
      "benchmark_expected_samples": 1, "sample_index": 1
    }}
  }]}
}
JSON

  run_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"measurement results require persisted configured tests"* ]]
}

@test "measurement mode accepts a complete comparison with semantic passes and misses" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": {"label": "codex-gpt-5.6-sol-standard"},
        "response": {"output": "Complete Sol answer"},
        "success": true,
        "latencyMs": 1250,
        "cost": 0.42,
        "tokenUsage": {
          "prompt": 120, "completion": 30, "total": 150, "cached": 20,
          "assertions": {
            "prompt": 80, "completion": 10, "total": 90, "cached": 15
          }
        },
        "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
        "testCase": {"vars": {
          "case_id": "comparison-case",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": [
            "codex-gpt-5.6-sol-standard",
            "codex-gpt-5.6-terra-standard"
          ],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }}
      },
      {
        "provider": {"label": "codex-gpt-5.6-terra-standard"},
        "response": {"output": "Complete Terra answer"},
        "success": false,
        "latencyMs": 980,
        "cost": 0.31,
        "tokenUsage": {
          "prompt": 100, "completion": 25, "total": 125, "cached": 10,
          "assertions": {
            "prompt": 70, "completion": 8, "total": 78, "cached": 12
          }
        },
        "error": "The response mentions a provider error without fully satisfying the semantic rubric.",
        "failureReason": 1,
        "gradingResult": {
          "pass": false,
          "score": 0.5,
          "reason": "The response mentions a provider error without fully satisfying the semantic rubric."
        },
        "testCase": {"vars": {
          "case_id": "comparison-case",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": [
            "codex-gpt-5.6-sol-standard",
            "codex-gpt-5.6-terra-standard"
          ],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }}
      }
    ]
  }
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"]'

  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "measurement mode does not apply plugin hard guards to a semantic rubric miss" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {"output": "Complete Sol answer"},
    "success": false,
    "latencyMs": 1250,
    "cost": 0.42,
    "tokenUsage": {
      "prompt": 120, "completion": 30, "total": 150, "cached": 20,
      "assertions": {
        "prompt": 80, "completion": 10, "total": 90, "cached": 15
      }
    },
    "error": "Response appears incomplete under the semantic rubric.",
    "failureReason": 1,
    "gradingResult": {
      "pass": false,
      "score": 0.5,
      "reason": "Response appears incomplete under the semantic rubric."
    },
    "testCase": {"vars": {
      "case_id": "comparison-case", "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
      "benchmark_expected_samples": 1, "sample_index": 1
    }}
  }]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard"]'

  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "measurement mode rejects a missing expected provider sample row" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {"output": "Complete Sol answer"},
    "success": true,
    "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
    "testCase": {"vars": {
      "case_id": "comparison-case",
      "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": [
        "codex-gpt-5.6-sol-standard",
        "codex-gpt-5.6-terra-standard"
      ],
      "benchmark_expected_samples": 1,
      "sample_index": 1
    }}
  }]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"missing expected result"* ]]
}

@test "measurement mode rejects an entirely omitted configured case" {
  cat >"$RESULTS" <<'JSON'
{
  "config": {
    "tests": [
      {
        "providers": ["codex-gpt-5.6-sol-standard"],
        "vars": {
          "case_id": "case-a",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }
      },
      {
        "providers": ["codex-gpt-5.6-sol-standard"],
        "vars": {
          "case_id": "case-b",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }
      }
    ]
  },
  "results": {
    "results": [
      {
        "provider": {"label": "codex-gpt-5.6-sol-standard"},
        "response": {"output": "Complete answer"},
        "success": true,
        "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
        "testCase": {"vars": {
          "case_id": "case-a",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }}
      }
    ]
  }
}
JSON

  run_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"case-b: missing expected result for provider codex-gpt-5.6-sol-standard, sample 1"* ]]
}

@test "measurement mode rejects a result case absent from persisted config" {
  cat >"$RESULTS" <<'JSON'
{
  "config": {
    "tests": [{
      "providers": ["codex-gpt-5.6-sol-standard"],
      "vars": {
        "case_id": "configured-case",
        "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
        "benchmark_expected_samples": 1,
        "sample_index": 1
      }
    }]
  },
  "results": {
    "results": [
      {
        "provider": {"label": "codex-gpt-5.6-sol-standard"},
        "response": {"output": "Configured answer"},
        "success": true,
        "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
        "testCase": {"vars": {
          "case_id": "configured-case",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }}
      },
      {
        "provider": {"label": "codex-gpt-5.6-sol-standard"},
        "response": {"output": "Unconfigured answer"},
        "success": true,
        "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
        "testCase": {"vars": {
          "case_id": "unconfigured-case",
          "min_pass_rate": 0,
          "value_gate_mode": "measurement",
          "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
          "benchmark_expected_samples": 1,
          "sample_index": 1
        }}
      }
    ]
  }
}
JSON

  run_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"unconfigured-case: result is absent from configured measurement tests"* ]]
}

@test "measurement mode rejects a duplicate provider sample row" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [
    {
      "provider": {"label": "codex-gpt-5.6-sol-standard"},
      "response": {"output": "First Sol answer"},
      "success": true,
      "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    },
    {
      "provider": {"label": "codex-gpt-5.6-sol-standard"},
      "response": {"output": "Duplicate Sol answer"},
      "success": true,
      "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    },
    {
      "provider": {"label": "codex-gpt-5.6-terra-standard"},
      "response": {"output": "Complete Terra answer"},
      "success": false,
      "error": "Semantic rubric miss",
      "failureReason": 1,
      "gradingResult": {"pass": false, "score": 0, "reason": "Semantic rubric miss"},
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    }
  ]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"duplicate result"* ]]
}

@test "measurement mode rejects a target provider response error" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [
    {
      "provider": {"label": "codex-gpt-5.6-sol-standard"},
      "response": {"error": "Codex turn failed: provider unavailable"},
      "success": false,
      "error": "Codex turn failed: provider unavailable",
      "failureReason": 2,
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    },
    {
      "provider": {"label": "codex-gpt-5.6-terra-standard"},
      "response": {"output": "Complete Terra answer"},
      "success": true,
      "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    }
  ]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"target provider error"* ]]
}

@test "measurement mode rejects a grader provider error" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [
    {
      "provider": {"label": "codex-gpt-5.6-sol-standard"},
      "response": {"output": "Complete Sol answer"},
      "success": false,
      "error": "Error calling grading provider: rate limit exceeded",
      "failureReason": 2,
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    },
    {
      "provider": {"label": "codex-gpt-5.6-terra-standard"},
      "response": {"output": "Complete Terra answer"},
      "success": true,
      "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
      "testCase": {"vars": {
        "case_id": "comparison-case", "min_pass_rate": 0,
        "value_gate_mode": "measurement",
        "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"],
        "benchmark_expected_samples": 1, "sample_index": 1
      }}
    }
  ]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard", "codex-gpt-5.6-terra-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"grader error"* ]]
}

@test "measurement mode rejects malformed grader evidence" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {"output": "Complete Sol answer"},
    "success": false,
    "gradingResult": {},
    "testCase": {"vars": {
      "case_id": "comparison-case", "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
      "benchmark_expected_samples": 1, "sample_index": 1
    }}
  }]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"malformed grader result"* ]]
}

@test "measurement mode rejects a row without target output" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {},
    "success": false,
    "error": "No output",
    "failureReason": 0,
    "testCase": {"vars": {
      "case_id": "comparison-case", "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
      "benchmark_expected_samples": 1, "sample_index": 1
    }}
  }]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"missing target output"* ]]
}

@test "measurement mode rejects malformed expectation metadata" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-gpt-5.6-sol-standard"},
    "response": {"output": "Complete Sol answer"},
    "success": true,
    "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
    "testCase": {"vars": {
      "case_id": "comparison-case", "min_pass_rate": 0,
      "value_gate_mode": "measurement",
      "benchmark_expected_provider_labels": ["codex-gpt-5.6-sol-standard"],
      "benchmark_expected_samples": 0, "sample_index": 1
    }}
  }]}
}
JSON

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard"]'

  [ "$status" -eq 1 ]
  [[ "$output" == *"malformed measurement metadata"* ]]
}

@test "measurement mode accepts complete paid-provider measurement metrics" {
  write_valid_paid_measurement_artifact

  run_configured_checker \
    "comparison-case" \
    '["codex-gpt-5.6-sol-standard"]'

  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "measurement mode rejects aggregate grader direct errors" {
  local mutation

  for mutation in \
    '.results.results[0].gradingResult.error = "grader failed"' \
    '.results.results[0].gradingResult.providerError = "grader provider failed"'; do
    assert_paid_measurement_mutation_rejected "$mutation" "grader error"
  done
}

@test "measurement mode rejects aggregate grader metadata errors" {
  local mutation

  for mutation in \
    '.results.results[0].gradingResult.metadata.graderError = "grader failed"' \
    '.results.results[0].gradingResult.metadata.error = "grader failed"' \
    '.results.results[0].gradingResult.metadata.providerError = "grader provider failed"'; do
    assert_paid_measurement_mutation_rejected "$mutation" "grader error"
  done
}

@test "measurement mode rejects component grader direct errors" {
  local mutation

  for mutation in \
    '.results.results[0].gradingResult.componentResults[0].error = "grader failed"' \
    '.results.results[0].gradingResult.componentResults[0].providerError = "grader provider failed"'; do
    assert_paid_measurement_mutation_rejected "$mutation" "grader error"
  done
}

@test "measurement mode rejects component grader metadata and response errors" {
  local mutation

  for mutation in \
    '.results.results[0].gradingResult.componentResults[0].metadata.graderError = "grader failed"' \
    '.results.results[0].gradingResult.componentResults[0].metadata.error = "grader failed"' \
    '.results.results[0].gradingResult.componentResults[0].metadata.providerError = "grader provider failed"' \
    '.results.results[0].gradingResult.componentResults[0].response.error = "grader failed"' \
    '.results.results[0].gradingResult.componentResults[0].response.providerError = "grader provider failed"'; do
    assert_paid_measurement_mutation_rejected "$mutation" "grader error"
  done
}

@test "measurement mode rejects nested component grader errors" {
  assert_paid_measurement_mutation_rejected \
    '.results.results[0].gradingResult.componentResults[0].componentResults = [{componentResults: [{metadata: {graderError: "nested grader failed"}}]}]' \
    "grader error"
}

@test "measurement mode rejects a target response provider error" {
  assert_paid_measurement_mutation_rejected \
    '.results.results[0].response.providerError = "target provider failed"' \
    "target provider error"
}

@test "measurement mode rejects missing or malformed latency" {
  local mutation

  for mutation in \
    'del(.results.results[0].latencyMs)' \
    '.results.results[0].latencyMs = "1250"' \
    '.results.results[0].latencyMs = 0' \
    '.results.results[0].latencyMs = -1'; do
    assert_paid_measurement_mutation_rejected \
      "$mutation" \
      "invalid measurement latency"
  done
}

@test "measurement mode rejects missing or malformed target token usage" {
  local mutation

  for mutation in \
    'del(.results.results[0].tokenUsage.prompt)' \
    '.results.results[0].tokenUsage.prompt = "120"' \
    '.results.results[0].tokenUsage.completion = -1' \
    '.results.results[0].tokenUsage.total = 149' \
    '.results.results[0].tokenUsage.cached = 121' \
    '.results.results[0].tokenUsage = {prompt: 0, completion: 0, total: 0, cached: 0, assertions: .results.results[0].tokenUsage.assertions}' \
    'del(.results.results[0].tokenUsage)'; do
    assert_paid_measurement_mutation_rejected \
      "$mutation" \
      "invalid target token usage"
  done
}

@test "measurement mode rejects missing or malformed grader token usage" {
  local mutation

  for mutation in \
    'del(.results.results[0].tokenUsage.assertions.prompt)' \
    '.results.results[0].tokenUsage.assertions.prompt = "80"' \
    '.results.results[0].tokenUsage.assertions.completion = -1' \
    '.results.results[0].tokenUsage.assertions.total = 89' \
    '.results.results[0].tokenUsage.assertions.cached = 81' \
    '.results.results[0].tokenUsage.assertions = {prompt: 0, completion: 0, total: 0, cached: 0}' \
    'del(.results.results[0].tokenUsage.assertions)'; do
    assert_paid_measurement_mutation_rejected \
      "$mutation" \
      "invalid grader token usage"
  done
}

@test "measurement mode rejects missing or malformed paid-provider cost" {
  local mutation

  for mutation in \
    'del(.results.results[0].cost)' \
    '.results.results[0].cost = "0.42"' \
    '.results.results[0].cost = 0' \
    '.results.results[0].cost = -0.01'; do
    assert_paid_measurement_mutation_rejected \
      "$mutation" \
      "invalid measurement cost"
  done
}

@test "non-measurement rows do not require benchmark metrics" {
  cat >"$RESULTS" <<'JSON'
{
  "results": {"results": [{
    "provider": {"label": "codex-standard-targeted-plugins"},
    "response": {"output": "Complete answer"},
    "success": true,
    "gradingResult": {"pass": true, "score": 1, "reason": "Rubric satisfied"},
    "testCase": {"vars": {
      "case_id": "standard-case", "min_pass_rate": 1,
      "value_gate_mode": "none"
    }}
  }]}
}
JSON

  run_checker

  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}
