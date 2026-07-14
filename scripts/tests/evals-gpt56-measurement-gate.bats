#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RESULTS="$(mktemp)"
  BENCHMARK_CONFIG="$ROOT/evals/benchmarks/gpt-5.6-model-family/promptfooconfig.yaml"
}

teardown() {
  rm -f "$RESULTS"
}

run_checker() {
  run node "$ROOT/scripts/evals/check-gpt56-measurement.mjs" "$RESULTS"
}

run_expected_benchmark_checker() {
  run env \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="${RESULTS}.skills-home" \
    CODEX_EVAL_HOME_NO_PLUGINS="${RESULTS}.no-plugins-home" \
    GPT56_BENCHMARK_WORKSPACE="${RESULTS}.workspace" \
    GPT56_BENCHMARK_SAMPLES="${EXPECTED_BENCHMARK_SAMPLES:-1}" \
    PROMPTFOO_MAX_CONCURRENCY="${EXPECTED_MAX_CONCURRENCY:-2}" \
    node "$ROOT/scripts/evals/check-gpt56-measurement.mjs" \
    "$RESULTS" \
    --expected-measurement-config "$BENCHMARK_CONFIG"
}

write_expected_benchmark_artifact() {
  local mutation="${1:-complete}"
  local samples="${2:-2}"
  EXPECTED_BENCHMARK_SAMPLES="$samples"

  node - "$BENCHMARK_CONFIG" "$RESULTS" "$samples" "$mutation" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const { parse } = require('yaml');

const configPath = process.argv[2];
const resultsPath = process.argv[3];
const samples = Number(process.argv[4]);
const mutation = process.argv[5];
const benchmarkDir = path.dirname(configPath);

process.env.CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE = `${resultsPath}.skills-home`;
process.env.CODEX_EVAL_HOME_NO_PLUGINS = `${resultsPath}.no-plugins-home`;
process.env.GPT56_BENCHMARK_WORKSPACE = `${resultsPath}.workspace`;
process.env.GPT56_BENCHMARK_SAMPLES = String(samples);
const source = parse(fs.readFileSync(configPath, 'utf8'));
const resolveEnv = (value) => {
  if (Array.isArray(value)) return value.map(resolveEnv);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, resolveEnv(entry)]),
    );
  }
  if (typeof value !== 'string') return value;
  const match = value.match(/^\{\{\s*env\.([A-Za-z_][A-Za-z0-9_]*)\s*\}\}$/);
  return match ? process.env[match[1]] : value;
};
const resolvedSource = resolveEnv(source);
const configuredTests = require(path.join(benchmarkDir, 'cases.cjs'))();
const grader = resolvedSource.defaultTest.options.provider.text;
const resolvedGrader = {
  options: {
    id: grader.id,
    config: {
      ...grader.config,
      basePath: benchmarkDir,
    },
  },
  label: grader.label,
};
const promptConfig = { provider: { text: resolvedGrader } };
const promptTemplate = resolvedSource.prompts[0];
const config = {
  tags: {},
  description: resolvedSource.description,
  prompts: resolvedSource.prompts,
  providers: resolvedSource.providers,
  tests: configuredTests,
  env: {},
  defaultTest: {
    ...resolvedSource.defaultTest,
    vars: {},
    assert: [],
    metadata: {},
  },
  outputPath: [
    resultsPath,
    `${resultsPath}.html`,
    `${resultsPath}.xml`,
  ],
  extensions: [],
  metadata: resolvedSource.metadata,
  evaluateOptions: {},
};
const results = configuredTests.flatMap((testCase) =>
  testCase.providers.map((provider) => ({
    provider: {
      id: resolvedSource.providers.find((entry) => entry.label === provider).id,
      label: provider,
    },
    prompt: {
      raw: promptTemplate.replace(
        /\{\{\s*scenario_prompt\s*\}\}/g,
        testCase.vars.scenario_prompt,
      ),
      label: promptTemplate,
      config: promptConfig,
    },
    response: { output: `Complete answer for ${testCase.vars.case_id}` },
    success: true,
    latencyMs: 1250,
    cost: 0.42,
    tokenUsage: {
      prompt: 120,
      completion: 30,
      total: 150,
      cached: 20,
      assertions: {
        prompt: 80,
        completion: 10,
        total: 90,
        cached: 15,
      },
    },
    gradingResult: {
      pass: true,
      score: 1,
      reason: 'Rubric satisfied',
      componentResults: [{
        pass: true,
        score: 1,
        reason: 'Rubric satisfied',
      }],
    },
    vars: {
      ...testCase.vars,
      sessionId: `session-${testCase.vars.case_id}-${testCase.vars.sample_index}`,
    },
    testCase: {
      ...JSON.parse(JSON.stringify(testCase)),
      vars: {
        ...testCase.vars,
        sessionId: `session-${testCase.vars.case_id}-${testCase.vars.sample_index}`,
      },
      options: promptConfig,
      metadata: {},
    },
  })),
);
const runtimeOptions = {
  eventSource: 'cli',
  showProgressBar: true,
  repeat: 1,
  maxConcurrency: 2,
  cache: false,
};
let shareableUrl = null;

if (mutation === 'self-declared-one-row') {
  configuredTests.splice(0, configuredTests.length, configuredTests[0]);
  results.splice(1);
  const onlyProvider = results[0].provider.label;
  configuredTests[0].providers = [onlyProvider];
  configuredTests[0].vars.benchmark_expected_provider_labels = [onlyProvider];
  configuredTests[0].vars.benchmark_expected_samples = 1;
  results[0].testCase.providers = [onlyProvider];
  results[0].testCase.vars.benchmark_expected_provider_labels = [onlyProvider];
  results[0].testCase.vars.benchmark_expected_samples = 1;
}
if (mutation === 'configless') {
  configuredTests.splice(0);
}
if (mutation === 'missing-rows') {
  const omittedCaseId = configuredTests.at(-1).vars.case_id;
  for (let index = results.length - 1; index >= 0; index -= 1) {
    if (results[index].testCase.vars.case_id === omittedCaseId) {
      results.splice(index, 1);
    }
  }
  for (let index = configuredTests.length - 1; index >= 0; index -= 1) {
    if (configuredTests[index].vars.case_id === omittedCaseId) {
      configuredTests.splice(index, 1);
    }
  }
}
if (mutation === 'duplicate-row') {
  results.push(JSON.parse(JSON.stringify(results[0])));
}
if (mutation === 'extra-unmarked-row') {
  results.push({
    provider: { label: 'unmarked-extra-provider' },
    response: { output: 'This row is not part of the benchmark contract.' },
    success: true,
    gradingResult: { pass: true, score: 1, reason: 'Unmarked extra row' },
    testCase: {
      vars: {
        case_id: 'unmarked-extra-case',
        min_pass_rate: 0,
        value_gate_mode: 'none',
      },
    },
  });
}
if (mutation === 'missing-required-metrics') {
  delete results[0].latencyMs;
  delete results[0].cost;
  delete results[0].tokenUsage;
}
if (mutation === 'unexpected-config-fields') {
  configuredTests[0].provider = 'unexpected-provider';
  configuredTests[0].options = { prefix: 'unexpected active Promptfoo option' };
}
if (mutation === 'altered-persisted-config') {
  config.prompts = ['Different top-level prompt'];
  config.providers = [{ id: 'echo', label: 'not-the-configured-provider' }];
  config.defaultTest = {
    options: { provider: { text: { id: 'echo' } } },
  };
}
if (mutation === 'altered-result-contract') {
  for (const result of results) {
    result.provider.id = 'echo';
    result.prompt.label = 'Different prompt label';
    result.prompt.raw = 'Different rendered prompt';
    result.prompt.config = { provider: { text: { id: 'echo' } } };
    result.testCase.vars.scenario_prompt =
      'Different request than the canonical benchmark';
    result.testCase.assert = [{ type: 'llm-rubric', value: 'Always pass.' }];
    result.testCase.options = {
      provider: { text: { id: 'echo' } },
    };
  }
}
if (mutation === 'altered-runtime-event-source') {
  runtimeOptions.eventSource = 'web';
}
if (mutation === 'altered-runtime-progress') {
  runtimeOptions.showProgressBar = false;
}
if (mutation === 'altered-runtime-repeat') {
  runtimeOptions.repeat = 2;
}
if (mutation === 'extra-runtime-option') {
  runtimeOptions.filterPattern = 'only-one-row';
}
if (mutation === 'shared-artifact') {
  shareableUrl = 'https://example.invalid/shared-eval';
}

fs.writeFileSync(
  resultsPath,
  JSON.stringify({
    config,
    shareableUrl,
    metadata: { promptfooVersion: '0.121.18' },
    runtimeOptions,
    results: { results },
  }),
);
NODE
}

@test "pure GPT-5.6 measurement validation keeps failures and comparison state invocation-local" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { validateMeasurementArtifact } = await import(
  pathToFileURL(`${root}/scripts/evals/gpt56-measurement-contract.mjs`)
);

const vars = {
  case_id: 'comparison-case',
  min_pass_rate: 0,
  value_gate_mode: 'measurement',
  benchmark_expected_provider_labels: ['codex-gpt-5.6-sol-standard'],
  benchmark_expected_samples: 1,
  sample_index: 1,
};
const configuredTest = {
  providers: ['codex-gpt-5.6-sol-standard'],
  vars,
};
const validResult = {
  provider: { label: 'codex-gpt-5.6-sol-standard' },
  response: { output: 'Complete Sol answer' },
  success: true,
  latencyMs: 1250,
  cost: 0.42,
  tokenUsage: {
    prompt: 120,
    completion: 30,
    total: 150,
    cached: 20,
    assertions: {
      prompt: 80,
      completion: 10,
      total: 90,
      cached: 15,
    },
  },
  gradingResult: {
    pass: true,
    score: 1,
    reason: 'Rubric satisfied',
  },
  testCase: { vars },
};
const artifactFor = (result) => ({
  config: { tests: [configuredTest] },
  results: { results: [result] },
});
const invalidArtifact = artifactFor({
  ...validResult,
  response: {},
});
const validArtifact = artifactFor(validResult);
const invalidBefore = JSON.stringify(invalidArtifact);
const validBefore = JSON.stringify(validArtifact);

const invalidFailures = validateMeasurementArtifact({
  artifact: invalidArtifact,
  results: invalidArtifact.results.results,
  resultsPath: '/tmp/results.json',
  workingDirectory: '/tmp',
});
const validFailures = validateMeasurementArtifact({
  artifact: validArtifact,
  results: validArtifact.results.results,
  resultsPath: '/tmp/results.json',
  workingDirectory: '/tmp',
});

if (!invalidFailures.some((failure) => failure.includes('missing target output'))) {
  throw new Error(`invalid artifact was not rejected: ${JSON.stringify(invalidFailures)}`);
}
if (validFailures.length !== 0) {
  throw new Error(`valid artifact inherited failures: ${JSON.stringify(validFailures)}`);
}
if (
  JSON.stringify(invalidArtifact) !== invalidBefore ||
  JSON.stringify(validArtifact) !== validBefore
) {
  throw new Error('pure validator mutated an input artifact');
}
NODE

  [ "$status" -eq 0 ]
}

@test "expected benchmark measurement contract accepts four cases by three providers by configured samples" {
  write_expected_benchmark_artifact complete 2

  run_expected_benchmark_checker

  [ "$status" -eq 0 ]
  [[ "$output" == *"GPT-5.6 measurement contract passed"* ]]
}

@test "expected benchmark measurement contract rejects a self-declared one-row artifact" {
  write_expected_benchmark_artifact self-declared-one-row 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
}

@test "expected benchmark measurement contract rejects a configless artifact" {
  write_expected_benchmark_artifact configless 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"persisted configured tests"* ]]
}

@test "expected benchmark measurement contract rejects missing configured rows" {
  write_expected_benchmark_artifact missing-rows 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
}

@test "expected benchmark measurement contract rejects duplicate result rows" {
  write_expected_benchmark_artifact duplicate-row 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"duplicate result"* ]]
}

@test "expected benchmark measurement contract rejects extra unmarked result rows" {
  write_expected_benchmark_artifact extra-unmarked-row 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"unexpected unmarked result"* ]]
}

@test "expected benchmark measurement contract rejects rows missing required metrics" {
  write_expected_benchmark_artifact missing-required-metrics 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"invalid measurement latency"* ]]
  [[ "$output" == *"invalid target token usage"* ]]
  [[ "$output" == *"invalid grader token usage"* ]]
  [[ "$output" == *"invalid measurement cost"* ]]
}

@test "expected benchmark measurement contract rejects unexpected active config fields" {
  write_expected_benchmark_artifact unexpected-config-fields 2

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"persisted configured measurement test differs from canonical contract"* ]]
}

@test "expected benchmark measurement contract rejects altered persisted config independently" {
  write_expected_benchmark_artifact altered-persisted-config 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.config.prompts"* ]]
  [[ "$output" == *"artifact.config.providers"* ]]
  [[ "$output" == *"artifact.config.defaultTest"* ]]
}

@test "expected benchmark measurement contract rejects altered result semantics independently" {
  write_expected_benchmark_artifact altered-result-contract 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.results.results[0].provider"* ]]
  [[ "$output" == *"artifact.results.results[0].prompt"* ]]
  [[ "$output" == *"artifact.results.results[0].testCase"* ]]
}

@test "expected benchmark measurement contract rejects a non-CLI runtime source" {
  write_expected_benchmark_artifact altered-runtime-event-source 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.runtimeOptions.eventSource"* ]]
}

@test "expected benchmark measurement contract rejects runtime repetition" {
  write_expected_benchmark_artifact altered-runtime-repeat 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.runtimeOptions.repeat"* ]]
}

@test "expected benchmark measurement contract rejects altered progress mode" {
  write_expected_benchmark_artifact altered-runtime-progress 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.runtimeOptions.showProgressBar"* ]]
}

@test "expected benchmark measurement contract rejects extra runtime filters" {
  write_expected_benchmark_artifact extra-runtime-option 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.runtimeOptions keys"* ]]
}

@test "expected benchmark measurement contract rejects hosted sharing" {
  write_expected_benchmark_artifact shared-artifact 1

  run_expected_benchmark_checker

  [ "$status" -eq 1 ]
  [[ "$output" == *"artifact.shareableUrl"* ]]
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
  [[ "$output" == *"GPT-5.6 measurement contract passed"* ]]
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
  [[ "$output" == *"GPT-5.6 measurement contract passed"* ]]
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
  [[ "$output" == *"GPT-5.6 measurement contract passed"* ]]
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
  [[ "$output" == *"GPT-5.6 measurement contract passed"* ]]
}
