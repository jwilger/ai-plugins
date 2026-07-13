#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  BENCHMARK_CASES="$ROOT/evals/benchmarks/gpt-5.6-model-family/cases.cjs"
  BENCHMARK_RUNNER="$ROOT/scripts/evals/run-gpt56-benchmark.sh"
}

assert_sample_value_rejected() {
  local value="$1"

  run env GPT56_BENCHMARK_SAMPLES="$value" node - "$BENCHMARK_CASES" <<'NODE'
const loadCases = require(process.argv[2]);
loadCases();
NODE

  [ "$status" -ne 0 ]
  [[ "$output" == *"GPT56_BENCHMARK_SAMPLES must be a canonical integer from 1 through 10"* ]]

  run env GPT56_BENCHMARK_SAMPLES="$value" "$BENCHMARK_RUNNER" --dry-run

  [ "$status" -eq 2 ]
  [[ "$output" == *"GPT56_BENCHMARK_SAMPLES must be a canonical integer from 1 through 10"* ]]
}

@test "GPT-5.6 benchmark defaults to one measured sample per case" {
  run env -u GPT56_BENCHMARK_SAMPLES node - "$BENCHMARK_CASES" <<'NODE'
const loadCases = require(process.argv[2]);
const cases = loadCases();

if (cases.length !== 4) {
  throw new Error(`expected four cases, got ${cases.length}`);
}

for (const testCase of cases) {
  const vars = testCase.vars;
  if (vars.sample_index !== 1 || vars.benchmark_expected_samples !== 1) {
    throw new Error(`${testCase.description}: unexpected sample metadata ${JSON.stringify(vars)}`);
  }
  if (vars.min_pass_rate !== 0 || vars.value_gate_mode !== 'measurement') {
    throw new Error(`${testCase.description}: not configured as a measurement`);
  }
  if (JSON.stringify(vars.benchmark_expected_provider_labels) !== JSON.stringify(testCase.providers)) {
    throw new Error(`${testCase.description}: expected provider labels do not match the case providers`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 benchmark accepts two measured samples per case" {
  run env GPT56_BENCHMARK_SAMPLES=2 node - "$BENCHMARK_CASES" <<'NODE'
const loadCases = require(process.argv[2]);
const cases = loadCases();

if (cases.length !== 8) {
  throw new Error(`expected eight cases, got ${cases.length}`);
}

for (const testCase of cases) {
  const vars = testCase.vars;
  if (vars.benchmark_expected_samples !== 2) {
    throw new Error(`${testCase.description}: expected sample count is not two`);
  }
  if (vars.min_pass_rate !== 0 || vars.value_gate_mode !== 'measurement') {
    throw new Error(`${testCase.description}: not configured as a measurement`);
  }
  if (JSON.stringify(vars.benchmark_expected_provider_labels) !== JSON.stringify(testCase.providers)) {
    throw new Error(`${testCase.description}: expected provider labels do not match the case providers`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 benchmark rejects malformed sample counts" {
  assert_sample_value_rejected "not-a-number"
}

@test "GPT-5.6 benchmark rejects a zero sample count" {
  assert_sample_value_rejected "0"
}

@test "GPT-5.6 benchmark rejects a negative sample count" {
  assert_sample_value_rejected "-1"
}

@test "GPT-5.6 benchmark rejects noncanonical integer forms" {
  local value
  for value in "01" "+1" "1.0" "1e0" " 1" "1 "; do
    assert_sample_value_rejected "$value"
  done
}

@test "GPT-5.6 benchmark rejects sample counts above ten" {
  assert_sample_value_rejected "11"
}

@test "GPT-5.6 benchmark help explains the bounded per-sample model-turn cost" {
  run "$BENCHMARK_RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"GPT56_BENCHMARK_SAMPLES"* ]]
  [[ "$output" == *"supported range 1-10"* ]]
  [[ "$output" == *"4 cases x 3 execution providers x 1 grader per output"* ]]
  [[ "$output" == *"24 model turns per sample"* ]]
}
