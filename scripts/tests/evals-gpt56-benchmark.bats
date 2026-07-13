#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  git_common_dir="$(cd "$ROOT" && cd "$(git rev-parse --git-common-dir)" && pwd -P)"
  MAIN_CHECKOUT="$(cd "$git_common_dir/.." && pwd -P)"
  BENCHMARK_RUNNER="$ROOT/scripts/evals/run-gpt56-benchmark.sh"
  CALIBRATION_CHECKER="$ROOT/scripts/evals/check-gpt56-grader-calibration.mjs"
}

write_calibration_artifact() {
  local artifact="$1"
  local mutation="$2"
  local workspace="${3:-$ROOT/.dependencies/evals/agent-workspace}"
  local no_plugins_home="${4:-$ROOT/.dependencies/evals/codex-home-no-plugins}"

  node - "$ROOT" "$artifact" "$mutation" "$workspace" "$no_plugins_home" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');

const root = process.argv[2];
const artifact = process.argv[3];
const mutation = process.argv[4];
const workspace = process.argv[5];
const noPluginsHome = process.argv[6];
const loadCases = require(
  path.join(root, 'evals/benchmarks/gpt-5.6-model-family/grader-cases.cjs'),
);

const configuredProvider = (label) => ({
  providerId: 'file://trace-enforced-codex-provider.mjs',
  label,
  config: {
    model: label.replace(/^grader-/, '').replace(/-high$/, ''),
    model_reasoning_effort: 'high',
    working_dir: workspace,
    deep_tracing: false,
    skip_git_repo_check: true,
    cli_config: { features: { plugins: false } },
    cli_env: {
      CODEX_HOME: noPluginsHome,
    },
  },
});

const results = loadCases().map((testCase) => {
  const assertions = testCase.assert.map((assertion) => ({
    ...assertion,
    provider: configuredProvider(assertion.provider),
  }));
  const namedScores = Object.fromEntries(
    testCase.assert.map((assertion) => [assertion.metric, 1]),
  );

  return {
    success: true,
    score: 1,
    namedScores,
    provider: { id: 'echo', label: 'frozen-human-answer' },
    response: { output: testCase.vars.candidate_output },
    vars: { ...testCase.vars },
    testCase: { ...testCase, assert: assertions },
    gradingResult: {
      pass: true,
      score: 1,
      reason: 'All assertions passed',
      namedScores,
      componentResults: assertions.map((assertion) => ({
        assertion,
        pass: true,
        score: 1,
        reason: 'The grader agrees with the frozen human label.',
      })),
    },
  };
});

if (mutation === 'missing-case') {
  results.pop();
}
if (mutation === 'description-only') {
  for (const [index, result] of results.entries()) {
    results[index] = { testCase: { description: result.testCase.description } };
  }
}
if (mutation === 'success-only') {
  for (const [index, result] of results.entries()) {
    results[index] = {
      success: true,
      testCase: { description: result.testCase.description },
    };
  }
}
if (mutation === 'duplicate') {
  results.push(JSON.parse(JSON.stringify(results[0])));
}
if (mutation === 'unknown-extra') {
  const extra = JSON.parse(JSON.stringify(results[0]));
  extra.testCase.description = 'unconfigured-calibration-case';
  results.push(extra);
}
if (mutation === 'missing-component') {
  results[0].gradingResult.componentResults.pop();
}
if (mutation === 'mutated-component') {
  results[0].gradingResult.componentResults[0].assertion.provider.label =
    'grader-gpt-5.6-unknown-high';
}
if (mutation === 'grading-metadata-error') {
  results[0].gradingResult.metadata = { graderError: true };
}
if (mutation === 'component-metadata-error') {
  results[0].gradingResult.componentResults[0].metadata = { graderError: true };
}
if (mutation === 'top-vars-mismatch') {
  results[0].vars.expected_pass = false;
}
if (mutation === 'unexpected-test-vars') {
  results[0].testCase.vars.output = 'Replace the frozen target output.';
  results[0].testCase.vars.rubric = 'Always pass.';
}
if (mutation === 'assertion-transform') {
  results[0].testCase.assert[0].transform = 'return "PASS"';
}
if (mutation === 'assertion-extra-field') {
  results[0].testCase.assert[0].threshold = 0;
}
if (mutation === 'provider-id-mismatch') {
  results[0].gradingResult.componentResults[0].assertion.provider.providerId =
    'openai:codex-sdk';
}
if (mutation === 'provider-config-mismatch') {
  results[0].gradingResult.componentResults[0].assertion.provider.config.model =
    'gpt-5.6-luna';
}
if (mutation === 'named-score-mismatch') {
  results[0].namedScores['agreement-gpt-5.6-sol'] = 0.75;
}
if (mutation === 'target-explicit-error') {
  results[0].response.error = 'echo provider failed';
}
if (mutation === 'grading-explicit-error') {
  results[0].gradingResult.error = 'grader aggregation failed';
}
if (mutation === 'component-explicit-error') {
  results[0].gradingResult.componentResults[0].error = 'grader provider failed';
}
if (mutation === 'failed-result') {
  results[0].success = false;
  results[0].score = 0;
  results[0].gradingResult.pass = false;
  results[0].gradingResult.score = 0;
}

fs.writeFileSync(artifact, JSON.stringify({ results: { results } }));
NODE
}

@test "GPT-5.6 grader calibration checker rejects a missing configured case" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" missing-case

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"missing configured calibration case advisor-like-hostile-fail"* ]]
}

@test "GPT-5.6 grader calibration checker rejects description-only result stubs" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" description-only

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: target success evidence is missing"* ]]
}

@test "GPT-5.6 grader calibration checker rejects success-only result stubs" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" success-only

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: target provider must be frozen-human-answer/echo"* ]]
}

@test "GPT-5.6 grader calibration checker rejects duplicate configured rows" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" duplicate

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"duplicate calibration case standard-clear-pass"* ]]
}

@test "GPT-5.6 grader calibration checker rejects unknown additional rows" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" unknown-extra

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"unknown calibration case unconfigured-calibration-case"* ]]
}

@test "GPT-5.6 grader calibration checker rejects a missing grader component" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" missing-component

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: expected exactly 3 grader components, got 2"* ]]
}

@test "GPT-5.6 grader calibration checker rejects a mutated grader component" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" mutated-component

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: unexpected grader component grader-gpt-5.6-unknown-high / agreement-gpt-5.6-sol"* ]]
}

@test "GPT-5.6 grader calibration checker rejects grading metadata errors" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" grading-metadata-error

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: grading error at gradingResult.metadata.graderError"* ]]
}

@test "GPT-5.6 grader calibration checker rejects component metadata errors" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" component-metadata-error

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: grader component grader-gpt-5.6-sol-high has an error at component.metadata.graderError"* ]]
}

@test "GPT-5.6 grader calibration checker requires matching top-level vars" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" top-vars-mismatch

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: top-level result vars do not match configuration"* ]]
}

@test "GPT-5.6 grader calibration checker rejects unexpected output and rubric vars" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" unexpected-test-vars

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: persisted case vars do not exactly match configuration"* ]]
}

@test "GPT-5.6 grader calibration checker rejects assertion transforms" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" assertion-transform

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: persisted grader assertion has unexpected fields"* ]]
}

@test "GPT-5.6 grader calibration checker rejects other unexpected assertion fields" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" assertion-extra-field

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: persisted grader assertion has unexpected fields"* ]]
}

@test "GPT-5.6 grader calibration checker requires the configured grader provider ID" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" provider-id-mismatch

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: unexpected grader component provider identity"* ]]
}

@test "GPT-5.6 grader calibration checker requires the configured grader provider config" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" provider-config-mismatch

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: unexpected grader component provider identity"* ]]
}

@test "GPT-5.6 grader calibration checker requires named scores to match components" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" named-score-mismatch

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: named scores do not match grader component scores"* ]]
}

@test "GPT-5.6 grader calibration checker accepts a complete canonical artifact" {
  artifact="$(mktemp)"
  write_calibration_artifact "$artifact" complete

  run node "$CALIBRATION_CHECKER" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 0 ]
  [[ "$output" == *"verified 8 complete GPT-5.6 grader calibration results across 3 configured graders"* ]]
}

@test "GPT-5.6 grader calibration checker rejects canonical explicit errors" {
  artifact="$(mktemp)"

  for fixture in \
    "target-explicit-error|target/provider error at response.error" \
    "grading-explicit-error|grading error at gradingResult.error" \
    "component-explicit-error|has an error at component.error"; do
    mutation="${fixture%%|*}"
    expected="${fixture#*|}"
    write_calibration_artifact "$artifact" "$mutation"

    run node "$CALIBRATION_CHECKER" "$artifact"

    [ "$status" -eq 1 ]
    [[ "$output" == *"$expected"* ]]
  done

  rm -f "$artifact"
}

@test "Codex home preparation supports a skills-only marketplace cache" {
  temp_root="$(mktemp -d)"
  eval_home="$temp_root/codex-home"
  plugin_home="$eval_home/plugins/cache/ai-plugins/agentic-systems-engineering"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode skills-only-marketplace \
    --plugins agentic-systems-engineering,development-discipline

  [ "$status" -eq 0 ]
  version="$(jq -r '.version' "$ROOT/plugins/agentic-systems-engineering/.codex-plugin/plugin.json")"
  cached_plugin="$plugin_home/$version"
  [ -f "$cached_plugin/.codex-plugin/plugin.json" ]
  [ -d "$cached_plugin/skills" ]
  [ ! -e "$cached_plugin/.mcp.json" ]
  [ ! -e "$cached_plugin/bin" ]
  [ ! -e "$cached_plugin/.claude-plugin" ]
  [ ! -e "$cached_plugin/README.md" ]
  [ "$(grep -c '^\[plugins\.' "$eval_home/config.toml")" -eq 2 ]
  [ -d "$eval_home/plugins/cache/ai-plugins/development-discipline" ]
  [ ! -e "$eval_home/plugins/cache/ai-plugins/advisor" ]
  ! grep -q 'advisor@ai-plugins' "$eval_home/config.toml"
  grep -q '\[plugins\."agentic-systems-engineering@ai-plugins"\]' "$eval_home/config.toml"

  rm -rf "$temp_root"
}

@test "Codex home preparation removes stale eval state before rebuilding" {
  temp_root="$(mktemp -d)"
  eval_home="$temp_root/codex-home"
  mkdir -p \
    "$eval_home/sessions" \
    "$eval_home/plugins/cache/ai-plugins/stale-plugin/0.0.0" \
    "$eval_home/skills/stale-system-skill" \
    "$eval_home/apps/stale-app"
  printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
  printf 'stale\n' >"$eval_home/sessions/stale-session.jsonl"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  [ -f "$eval_home/config.toml" ]
  [ ! -e "$eval_home/sessions" ]
  [ ! -e "$eval_home/plugins" ]
  [ ! -e "$eval_home/skills" ]
  [ ! -e "$eval_home/apps" ]

  rm -rf "$temp_root"
}

@test "Codex home preparation preserves unowned and auth-overlapping paths" {
  temp_root="$(mktemp -d)"
  unowned_home="$temp_root/unowned"
  mkdir -p "$unowned_home"
  printf 'keep me\n' >"$unowned_home/user-file"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$unowned_home" \
    --plugin-mode no-plugins

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to replace unowned Codex eval home"* ]]
  [ -f "$unowned_home/user-file" ]

  auth_parent="$temp_root/auth-parent"
  auth_home="$auth_parent/auth-source"
  mkdir -p "$auth_home"
  printf 'ai-plugins Codex eval home\n' >"$auth_parent/.ai-plugins-eval-home"
  printf '{"token":"keep"}\n' >"$auth_home/auth.json"

  run env CODEX_EVAL_AUTH_HOME="$auth_home" node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$auth_parent" \
    --plugin-mode no-plugins

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing Codex eval home path that overlaps the auth source"* ]]
  grep -q 'keep' "$auth_home/auth.json"

  rm -rf "$temp_root"
}

@test "Codex home preparation refuses an unowned home with marketplace config" {
  temp_root="$(mktemp -d)"
  unowned_home="$temp_root/alternate-codex-home"
  mkdir -p "$unowned_home"
  printf '[marketplaces.ai-plugins]\nsource = "fixture"\n' >"$unowned_home/config.toml"
  printf 'keep me\n' >"$unowned_home/unrelated-session"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$unowned_home" \
    --plugin-mode no-plugins

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to replace unowned Codex eval home"* ]]
  grep -q 'keep me' "$unowned_home/unrelated-session"

  rm -rf "$temp_root"
}

@test "GPT-5.6 benchmark runner dry-run prepares each required Codex home and isolates artifacts" {
  temp_root="$(mktemp -d)"
  skills_home="$temp_root/skills-only-home"
  no_plugins_home="$temp_root/no-plugins-home"
  workspace="$temp_root/workspace"
  out_root="$temp_root/out"

  run env \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$skills_home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --dry-run --phase execution

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 2 ]
  [[ "$output" == *"$skills_home --plugin-mode skills-only-marketplace"* ]]
  [[ "$output" == *"--plugins agentic-systems-engineering\\,development-discipline"* ]]
  [[ "$output" == *"$no_plugins_home --plugin-mode no-plugins"* ]]
  [[ "$output" == *"$out_root/execution/results.json"* ]]
  [[ "$output" == *"--max-concurrency 2"* ]]
  [[ "$output" == *"check-gpt56-execution-isolation.mjs $out_root/execution/results.json"* ]]
  [ ! -e "$skills_home/config.toml" ]
  [ ! -e "$no_plugins_home/config.toml" ]

  run env \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --dry-run --phase grader-calibration

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 1 ]
  [[ "$output" == *"$no_plugins_home --plugin-mode no-plugins"* ]]
  [[ "$output" != *"--plugin-mode skills-only-marketplace"* ]]
  [[ "$output" == *"$out_root/grader-calibration/results.json"* ]]
  [[ "$output" != *"check-gpt56-execution-isolation.mjs"* ]]
  [[ "$output" == *"check-gpt56-grader-calibration.mjs $out_root/grader-calibration/results.json"* ]]
}

@test "GPT-5.6 benchmark runner rejects a concurrent live run before preparation while dry-run bypasses the lock" {
  temp_root="$(mktemp -d)"
  lock_path="$MAIN_CHECKOUT/.dependencies/evals/provider-eval.lock"
  preparation_marker="$temp_root/preparation-invoked"
  provider_marker="$temp_root/provider-invoked"
  fake_bin="$temp_root/bin"
  fake_promptfoo="$temp_root/promptfoo"
  mkdir -p "$fake_bin" "$(dirname "$lock_path")"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'echo "preparation stub invoked" >&2' \
    'touch "$PREPARATION_MARKER"' \
    'exit 91' \
    >"$fake_bin/node"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'touch "$PROVIDER_MARKER"' \
    'exit 92' \
    >"$fake_promptfoo"
  chmod +x "$fake_bin/node" "$fake_promptfoo"

  exec 8>>"$lock_path"
  flock --nonblock 8
  printf 'held by focused-run contention regression\n' >&8

  run env \
    PATH="$fake_bin:$PATH" \
    PREPARATION_MARKER="$preparation_marker" \
    PROVIDER_MARKER="$provider_marker" \
    PROMPTFOO_BIN="$fake_promptfoo" \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$temp_root/skills-home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
    GPT56_BENCHMARK_WORKSPACE="$temp_root/workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
    "$BENCHMARK_RUNNER" --phase execution

  [ "$status" -eq 75 ]
  [[ "$output" == *"provider-backed eval already active; lock is held: $lock_path"* ]]
  [ ! -e "$preparation_marker" ]
  [ ! -e "$provider_marker" ]
  [ ! -e "$temp_root/workspace" ]
  [ ! -e "$temp_root/skills-home" ]
  [ ! -e "$temp_root/no-plugins-home" ]
  [ ! -e "$temp_root/out" ]

  lock_contents_before="$(<"$lock_path")"
  run env \
    CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
    GPT56_BENCHMARK_WORKSPACE="$temp_root/workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
    "$BENCHMARK_RUNNER" --dry-run --phase grader-calibration
  lock_contents_after="$(<"$lock_path")"

  flock --unlock 8
  exec 8>&-
  rm -rf "$temp_root"

  [ "$status" -eq 0 ]
  [ "$lock_contents_after" = "$lock_contents_before" ]
}

@test "GPT-5.6 benchmark runner shares its provider lock across linked worktrees" {
  temp_root="$(mktemp -d)"
  fixture_main="$temp_root/main"
  fixture_worktree="$temp_root/linked"
  fake_bin="$temp_root/bin"
  preparation_marker="$temp_root/preparation-invoked"
  mkdir -p "$fixture_main/scripts/evals" "$fake_bin"
  cp "$BENCHMARK_RUNNER" "$fixture_main/scripts/evals/run-gpt56-benchmark.sh"

  git -C "$fixture_main" init -q
  git -C "$fixture_main" config user.name fixture
  git -C "$fixture_main" config user.email fixture@example.invalid
  git -C "$fixture_main" config commit.gpgSign false
  git -C "$fixture_main" add scripts/evals/run-gpt56-benchmark.sh
  git -C "$fixture_main" commit -qm fixture
  git -C "$fixture_main" worktree add -q --detach "$fixture_worktree"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'touch "$PREPARATION_MARKER"' \
    'exit 91' \
    >"$fake_bin/node"
  chmod +x "$fake_bin/node"

  lock_path="$fixture_main/.dependencies/evals/provider-eval.lock"
  mkdir -p "$(dirname "$lock_path")"
  exec 8>>"$lock_path"
  flock --nonblock 8

  run env \
    PATH="$fake_bin:$PATH" \
    PREPARATION_MARKER="$preparation_marker" \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$temp_root/skills-home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
    GPT56_BENCHMARK_WORKSPACE="$temp_root/workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
    "$fixture_worktree/scripts/evals/run-gpt56-benchmark.sh" --phase execution
  run_status="$status"
  run_output="$output"
  preparation_invoked=0
  [ ! -e "$preparation_marker" ] || preparation_invoked=1

  flock --unlock 8
  exec 8>&-
  rm -rf "$temp_root"

  [ "$run_status" -eq 75 ]
  [[ "$run_output" == *"provider-backed eval already active; lock is held: $lock_path"* ]]
  [ "$preparation_invoked" -eq 0 ]
}

@test "GPT-5.6 benchmark runner rejects concurrency outside the canonical one-to-two range" {
  temp_root="$(mktemp -d)"

  for concurrency in 0 -1 3 99 01 1.5 malformed ' 2'; do
    run env \
      PROMPTFOO_MAX_CONCURRENCY="$concurrency" \
      CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
      GPT56_BENCHMARK_WORKSPACE="$temp_root/workspace" \
      GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
      "$BENCHMARK_RUNNER" --dry-run --phase grader-calibration

    [ "$status" -eq 2 ]
    [[ "$output" == *"PROMPTFOO_MAX_CONCURRENCY must be 1 or 2"* ]]
  done

  rm -rf "$temp_root"
}

@test "GPT-5.6 benchmark runner defaults to a workspace outside the repository" {
  temp_root="$(mktemp -d)"

  run env -u GPT56_BENCHMARK_WORKSPACE \
    TMPDIR="$temp_root" \
    CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
    GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
    "$BENCHMARK_RUNNER" --dry-run --phase grader-calibration

  [ "$status" -eq 0 ]
  [[ "$output" == *"prepare-gpt56-workspace.mjs $temp_root/ai-plugins-gpt56-workspace-"* ]]
  [[ "$output" != *"$ROOT/.dependencies/evals/agent-workspace"* ]]
  [ ! -e "$temp_root"/ai-plugins-gpt56-workspace-* ]

  rm -rf "$temp_root"
}

@test "GPT-5.6 benchmark runner refuses workspace overlap with protected paths" {
  temp_root="$(mktemp -d)"
  skills_home="$temp_root/skills-home"
  no_plugins_home="$temp_root/no-plugins-home"
  out_root="$temp_root/out"
  auth_home="$temp_root/auth-home"

  for protected_kind in repository skills-home no-plugins-home output auth-home; do
    case "$protected_kind" in
      repository) workspace="$ROOT/.dependencies/evals/overlap-fixture-$$" ;;
      skills-home) workspace="$skills_home" ;;
      no-plugins-home) workspace="$no_plugins_home" ;;
      output) workspace="$out_root" ;;
      auth-home) workspace="$auth_home" ;;
    esac

    run env \
      CODEX_EVAL_AUTH_HOME="$auth_home" \
      CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$skills_home" \
      CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
      GPT56_BENCHMARK_WORKSPACE="$workspace" \
      GPT56_BENCHMARK_OUT_ROOT="$out_root" \
      "$BENCHMARK_RUNNER" --dry-run --phase execution

    [ "$status" -eq 2 ]
    [[ "$output" == *"GPT-5.6 benchmark workspace overlaps protected path"* ]]
  done

  [ ! -e "$ROOT/.dependencies/evals/overlap-fixture-$$" ]
  [ ! -e "$skills_home" ]
  [ ! -e "$no_plugins_home" ]
  [ ! -e "$out_root" ]
  [ ! -e "$auth_home" ]
  rm -rf "$temp_root"
}

@test "GPT-5.6 benchmark runner refuses an unowned nonempty workspace" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/unowned-workspace"
  promptfoo_marker="$temp_root/promptfoo-invoked"
  fake_promptfoo="$temp_root/promptfoo"
  mkdir -p "$workspace"
  printf 'keep me\n' >"$workspace/AGENTS.md"
  printf '#!/usr/bin/env bash\ntouch "$PROMPTFOO_MARKER"\nexit 23\n' >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    PROMPTFOO_BIN="$fake_promptfoo" \
    PROMPTFOO_MARKER="$promptfoo_marker" \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$temp_root/skills-home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
    "$BENCHMARK_RUNNER" --phase execution

  [ "$status" -eq 2 ]
  [[ "$output" == *"refusing to replace unowned GPT-5.6 benchmark workspace"* ]]
  grep -q 'keep me' "$workspace/AGENTS.md"
  [ ! -e "$promptfoo_marker" ]

  rm -rf "$temp_root"
}

@test "GPT-5.6 benchmark runner removes stale state from an owned workspace" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/owned-workspace"
  promptfoo_marker="$temp_root/promptfoo-invoked"
  fake_promptfoo="$temp_root/promptfoo"
  mkdir -p "$workspace"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' >"$workspace/.ai-plugins-gpt56-workspace"
  printf 'stale instructions\n' >"$workspace/AGENTS.md"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    '[ ! -e "$GPT56_BENCHMARK_WORKSPACE/AGENTS.md" ]' \
    'touch "$PROMPTFOO_MARKER"' \
    'exit 23' \
    >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    PROMPTFOO_BIN="$fake_promptfoo" \
    PROMPTFOO_MARKER="$promptfoo_marker" \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$temp_root/skills-home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$temp_root/no-plugins-home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$temp_root/out" \
    "$BENCHMARK_RUNNER" --phase execution

  [ "$status" -eq 23 ]
  [ -e "$promptfoo_marker" ]
  [ ! -e "$workspace/AGENTS.md" ]

  rm -rf "$temp_root"
}

@test "GPT-5.6 grader calibration runner enforces complete post-run artifacts" {
  temp_root="$(mktemp -d)"
  no_plugins_home="$temp_root/no-plugins-home"
  workspace="$temp_root/workspace"
  out_root="$temp_root/out"
  complete_artifact="$temp_root/complete.json"
  incomplete_artifact="$temp_root/incomplete.json"
  fake_promptfoo="$temp_root/promptfoo"

  write_calibration_artifact \
    "$complete_artifact" \
    complete \
    "$workspace" \
    "$no_plugins_home"
  write_calibration_artifact \
    "$incomplete_artifact" \
    missing-case \
    "$workspace" \
    "$no_plugins_home"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'mkdir -p "$EVAL_OUT_DIR"' \
    'cp "$CALIBRATION_ARTIFACT" "$EVAL_OUT_DIR/results.json"' \
    'exit "${PROMPTFOO_EXIT_STATUS:-0}"' \
    >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    CALIBRATION_ARTIFACT="$complete_artifact" \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --phase grader-calibration

  [ "$status" -eq 0 ]
  [[ "$output" == *"verified 8 complete GPT-5.6 grader calibration results"* ]]

  run env \
    OPENAI_API_KEY=fixture \
    CALIBRATION_ARTIFACT="$incomplete_artifact" \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --phase grader-calibration

  rm -rf "$temp_root"
  [ "$status" -eq 1 ]
  [[ "$output" == *"missing configured calibration case advisor-like-hostile-fail"* ]]
}

@test "GPT-5.6 grader calibration runner verifies artifacts after canonical runner failure" {
  temp_root="$(mktemp -d)"
  no_plugins_home="$temp_root/no-plugins-home"
  workspace="$temp_root/workspace"
  out_root="$temp_root/out"
  failed_artifact="$temp_root/failed.json"
  fake_promptfoo="$temp_root/promptfoo"

  write_calibration_artifact \
    "$failed_artifact" \
    failed-result \
    "$workspace" \
    "$no_plugins_home"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'mkdir -p "$EVAL_OUT_DIR"' \
    'cp "$CALIBRATION_ARTIFACT" "$EVAL_OUT_DIR/results.json"' \
    'exit 7' \
    >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    CALIBRATION_ARTIFACT="$failed_artifact" \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --phase grader-calibration

  rm -rf "$temp_root"
  [ "$status" -eq 1 ]
  [[ "$output" == *"standard-clear-pass: target success evidence is missing"* ]]
}

@test "GPT-5.6 grader calibration runner reports a missing post-run artifact" {
  temp_root="$(mktemp -d)"
  no_plugins_home="$temp_root/no-plugins-home"
  workspace="$temp_root/workspace"
  out_root="$temp_root/out"
  fake_promptfoo="$temp_root/promptfoo"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'exit 0' \
    >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --phase grader-calibration

  rm -rf "$temp_root"
  [ "$status" -eq 1 ]
  [[ "$output" == *"cannot read calibration artifact"* ]]
}

@test "GPT-5.6 grader calibration runner preserves terminal interruption without checking artifacts" {
  temp_root="$(mktemp -d)"
  no_plugins_home="$temp_root/no-plugins-home"
  workspace="$temp_root/workspace"
  out_root="$temp_root/out"
  complete_artifact="$temp_root/complete.json"
  fake_promptfoo="$temp_root/promptfoo"

  write_calibration_artifact \
    "$complete_artifact" \
    complete \
    "$workspace" \
    "$no_plugins_home"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'mkdir -p "$EVAL_OUT_DIR"' \
    'cp "$CALIBRATION_ARTIFACT" "$EVAL_OUT_DIR/results.json"' \
    'exit 130' \
    >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    CALIBRATION_ARTIFACT="$complete_artifact" \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --phase grader-calibration

  rm -rf "$temp_root"
  [ "$status" -eq 130 ]
  [[ "$output" == *"interrupted before completion with status 130"* ]]
  [[ "$output" != *"GPT-5.6 grader calibration verification"* ]]
  [[ "$output" != *"verified 8 complete GPT-5.6 grader calibration results"* ]]
}

@test "GPT-5.6 benchmark runner prepares isolated homes before invoking Promptfoo" {
  temp_root="$(mktemp -d)"
  skills_home="$temp_root/skills-only-home"
  no_plugins_home="$temp_root/no-plugins-home"
  workspace="$temp_root/workspace"
  out_root="$temp_root/out"
  fake_promptfoo="$temp_root/promptfoo"
  marker="$temp_root/promptfoo-invoked"
  expected_plugin="agentic-systems-engineering"
  agentic_version="$(jq -r '.version' "$ROOT/plugins/agentic-systems-engineering/.codex-plugin/plugin.json")"

  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'grep -q "\\[plugins\\.\\\"${EXPECTED_PLUGIN}@ai-plugins\\\"\\]" "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/config.toml"' \
    'grep -q "\\[plugins\\.\\\"development-discipline@ai-plugins\\\"\\]" "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/config.toml"' \
    '[ "$(grep -c "^\\[plugins\\." "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/config.toml")" -eq 2 ]' \
    '[ -d "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/plugins/cache/ai-plugins/$EXPECTED_PLUGIN" ]' \
    '[ -d "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/plugins/cache/ai-plugins/development-discipline" ]' \
    '[ ! -e "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/plugins/cache/ai-plugins/advisor" ]' \
    '! grep -q "advisor@ai-plugins" "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/config.toml"' \
    '[ ! -e "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE/plugins/cache/ai-plugins/agentic-systems-engineering/$AGENTIC_VERSION/.mcp.json" ]' \
    '! grep -q "^\\[plugins\\." "$CODEX_EVAL_HOME_NO_PLUGINS/config.toml"' \
    '[ -d "$GPT56_BENCHMARK_WORKSPACE" ]' \
    'mkdir -p "$EVAL_OUT_DIR"' \
    'node -e "require(\"node:fs\").writeFileSync(process.env.EVAL_OUT_DIR + \"/results.json\", JSON.stringify({results:{results:[{success:true,provider:{label:\"codex-gpt-5.6-terra-standard\"},testCase:{vars:{case_id:\"fixture\",sample_index:1}},response:{output:\"Direct answer\",raw:{finalResponse:\"Direct answer\",items:[{type:\"agent_message\",text:\"Direct answer\"}],notifications:[{method:\"rawResponseItem/completed\",params:{item:{type:\"message\",role:\"assistant\",content:[]}}}],serverRequests:[]}}}]}}))"' \
    'printf "%s\\n" "$*" >"$PROMPTFOO_MARKER"' \
    >"$fake_promptfoo"
  chmod +x "$fake_promptfoo"

  run env \
    OPENAI_API_KEY=fixture \
    AGENTIC_VERSION="$agentic_version" \
    EXPECTED_PLUGIN="$expected_plugin" \
    PROMPTFOO_MARKER="$marker" \
    PROMPTFOO_BIN="$fake_promptfoo" \
    EVAL_TIMEOUT=0 \
    CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE="$skills_home" \
    CODEX_EVAL_HOME_NO_PLUGINS="$no_plugins_home" \
    GPT56_BENCHMARK_WORKSPACE="$workspace" \
    GPT56_BENCHMARK_OUT_ROOT="$out_root" \
    "$BENCHMARK_RUNNER" --phase execution

  [ "$status" -eq 0 ]
  [ -s "$marker" ]
  [[ "$(<"$marker")" == *"promptfooconfig.yaml"* ]]
  [ -d "$skills_home/plugins/cache/ai-plugins/$expected_plugin" ]
  ! grep -q '^\[plugins\.' "$no_plugins_home/config.toml"

  rm -rf "$temp_root"
}

@test "GPT-5.6 model benchmark spans standard and advisor-like cases without installed Advisor routing" {
  run node - "$ROOT" <<'NODE'
const path = require('node:path');

const root = process.argv[2];
const loadCases = require(path.join(root, 'evals/benchmarks/gpt-5.6-model-family/cases.cjs'));
const cases = loadCases();

const standardPluginNames = loadCases.standardPluginNames?.();
const expectedStandardPluginNames = [
  'agentic-systems-engineering',
  'development-discipline',
];
if (JSON.stringify(standardPluginNames) !== JSON.stringify(expectedStandardPluginNames)) {
  throw new Error(`unexpected standard plugin scope: ${JSON.stringify(standardPluginNames)}`);
}
if (standardPluginNames.includes('advisor')) {
  throw new Error('standard plugin scope includes delegation-only Advisor guidance');
}

if (cases.length !== 4) {
  throw new Error(`expected four benchmark cases, got ${cases.length}`);
}

const categories = cases.reduce((counts, testCase) => {
  const category = testCase.vars.benchmark_category;
  counts[category] = (counts[category] || 0) + 1;
  return counts;
}, {});

if (categories.standard !== 2 || categories['advisor-like'] !== 2) {
  throw new Error(`unexpected category counts: ${JSON.stringify(categories)}`);
}

const advisorCaseIds = cases
  .filter((entry) => entry.vars.benchmark_category === 'advisor-like')
  .map((entry) => entry.vars.case_id)
  .sort();
const expectedAdvisorCaseIds = [
  'advisor-like-ticket-plan-outline',
  'advisor-like-tradeoff-recommendation',
];
if (JSON.stringify(advisorCaseIds) !== JSON.stringify(expectedAdvisorCaseIds)) {
  throw new Error(`unexpected advisor-like cases: ${JSON.stringify(advisorCaseIds)}`);
}

for (const testCase of cases.filter((entry) => entry.vars.benchmark_category === 'advisor-like')) {
  if (!testCase.vars.scenario_prompt.startsWith('Answer directly without delegating')) {
    throw new Error(`${testCase.description}: advisor-like prompt does not isolate the execution model`);
  }
  if (/\badvisor\b/i.test(testCase.vars.scenario_prompt)) {
    throw new Error(`${testCase.description}: advisor-like prompt still names Advisor`);
  }
  if (!testCase.providers.every((provider) => provider.endsWith('-advisor-like'))) {
    throw new Error(`${testCase.description}: advisor-like case uses a plugin-loaded provider`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 benchmark config isolates provider modes and uses Sol high grading" {
  run node - "$ROOT" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const { parse } = require('yaml');

const root = process.argv[2];
const config = parse(
  fs.readFileSync(
    path.join(root, 'evals/benchmarks/gpt-5.6-model-family/promptfooconfig.yaml'),
    'utf8',
  ),
);

const providers = config.providers;
if (config.tests !== 'file://cases.cjs') {
  throw new Error(`benchmark case loader is not config-relative: ${config.tests}`);
}
const expectedWorkspace = "{{ env.GPT56_BENCHMARK_WORKSPACE | default('../../../.dependencies/evals/agent-workspace') }}";
const models = [...new Set(providers.map((provider) => provider.config.model))].sort();
if (JSON.stringify(models) !== JSON.stringify(['gpt-5.6-luna', 'gpt-5.6-sol', 'gpt-5.6-terra'])) {
  throw new Error(`unexpected execution models: ${JSON.stringify(models)}`);
}

for (const provider of providers) {
  if (provider.config.working_dir !== expectedWorkspace) {
    throw new Error(`${provider.label}: workspace is not repository-root relative`);
  }
  if (provider.config.model_reasoning_effort !== 'medium') {
    throw new Error(`${provider.label}: execution effort is not medium`);
  }
  const standard = provider.label.endsWith('-standard');
  if (provider.config.cli_config?.features?.plugins !== standard) {
    throw new Error(`${provider.label}: plugin-skills choice does not match provider mode`);
  }
  const home = provider.config.cli_env.CODEX_HOME;
  if (standard && !home.includes('CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE')) {
    throw new Error(`${provider.label}: standard provider lacks the skills-only marketplace home`);
  }
  if (provider.label.endsWith('-advisor-like') && !home.includes('CODEX_EVAL_HOME_NO_PLUGINS')) {
    throw new Error(`${provider.label}: advisor-like provider lacks the no-plugin home`);
  }
}

const graderProvider = config.defaultTest.options.provider.text;
const grader = graderProvider.config;
if (grader.model !== 'gpt-5.6-sol' || grader.model_reasoning_effort !== 'high') {
  throw new Error(`unexpected grader: ${JSON.stringify(grader)}`);
}
if (grader.working_dir !== expectedWorkspace) {
  throw new Error('grader workspace is not repository-root relative');
}
if (!grader.cli_env.CODEX_HOME.includes('CODEX_EVAL_HOME_NO_PLUGINS')) {
  throw new Error('execution grader does not use the calibrated no-plugin home');
}
if (grader.cli_config?.features?.plugins !== false) {
  throw new Error('execution grader does not disable plugin loading');
}
NODE

  [ "$status" -eq 0 ]
}

@test "every focused GPT-5.6 model turn uses the trace-enforced app-server provider" {
  run node - "$ROOT" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const { parse } = require('yaml');

const root = process.argv[2];
const benchmarkDir = path.join(root, 'evals/benchmarks/gpt-5.6-model-family');
const execution = parse(
  fs.readFileSync(path.join(benchmarkDir, 'promptfooconfig.yaml'), 'utf8'),
);
const calibration = parse(
  fs.readFileSync(path.join(benchmarkDir, 'grader-promptfooconfig.yaml'), 'utf8'),
);
const expectedId = 'file://trace-enforced-codex-provider.mjs';
const executionTargets = execution.providers;
const executionGrader = execution.defaultTest.options.provider.text;
const calibrationGraders = calibration.providers.filter(
  (provider) => provider.label !== 'frozen-human-answer',
);
const modelProviders = [
  ...executionTargets,
  executionGrader,
  ...calibrationGraders,
];

if (modelProviders.length !== 10) {
  throw new Error(`expected ten model-provider entries, got ${modelProviders.length}`);
}
for (const provider of modelProviders) {
  if (provider.id !== expectedId) {
    throw new Error(`${provider.label || 'execution grader'} bypasses trace enforcement: ${provider.id}`);
  }
  if (Object.hasOwn(provider.config, 'enable_streaming')) {
    throw new Error(`${provider.label || 'execution grader'} retains SDK-only enable_streaming`);
  }
  if (typeof provider.config.cli_config?.features?.plugins !== 'boolean') {
    throw new Error(`${provider.label || 'execution grader'} lacks an explicit plugin feature choice`);
  }
}

for (const provider of executionTargets) {
  const standard = provider.label.endsWith('-standard');
  const expectedPlugins = standard;
  if (provider.config.cli_config.features.plugins !== expectedPlugins) {
    throw new Error(`${provider.label}: unexpected plugin feature choice`);
  }
  const home = provider.config.cli_env?.CODEX_HOME || '';
  const expectedHome = standard
    ? 'CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE'
    : 'CODEX_EVAL_HOME_NO_PLUGINS';
  if (!home.includes(expectedHome)) {
    throw new Error(`${provider.label}: unexpected Codex home ${home}`);
  }
}

for (const grader of [executionGrader, ...calibrationGraders]) {
  if (!grader.config.cli_env?.CODEX_HOME?.includes('CODEX_EVAL_HOME_NO_PLUGINS')) {
    throw new Error(`${grader.label || 'execution grader'} does not use the no-plugin home`);
  }
}

if (!fs.existsSync(path.join(benchmarkDir, 'trace-enforced-codex-provider.mjs'))) {
  throw new Error('trace-enforced provider module is missing');
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 grader calibration uses frozen human-labeled standard and advisor-like answers" {
  run node - "$ROOT" <<'NODE'
const path = require('node:path');

const root = process.argv[2];
const loadCases = require(
  path.join(root, 'evals/benchmarks/gpt-5.6-model-family/grader-cases.cjs'),
);
const cases = loadCases();

if (cases.length !== 8) {
  throw new Error(`expected eight grader calibration cases, got ${cases.length}`);
}

const families = [...new Set(cases.map((testCase) => testCase.vars.calibration_family))].sort();
if (JSON.stringify(families) !== JSON.stringify(['advisor-like', 'standard'])) {
  throw new Error(`unexpected calibration families: ${JSON.stringify(families)}`);
}

for (const family of families) {
  const labels = cases
    .filter((testCase) => testCase.vars.calibration_family === family)
    .map((testCase) => testCase.vars.human_label)
    .sort();
  if (JSON.stringify(labels) !== JSON.stringify(['borderline-fail', 'clear-fail', 'clear-pass', 'hostile-fail'])) {
    throw new Error(`${family}: unexpected human labels ${JSON.stringify(labels)}`);
  }
}

const hostileCases = cases.filter(
  (testCase) => testCase.vars.human_label === 'hostile-fail',
);
if (hostileCases.length !== 2) {
  throw new Error(`expected two hostile calibration cases, got ${hostileCases.length}`);
}
for (const testCase of hostileCases) {
  if (testCase.vars.expected_pass !== false) {
    throw new Error(`${testCase.description}: hostile answer is not human-labelled fail`);
  }
  if (!/\b(?:exec|wait)\b/.test(testCase.vars.candidate_output)) {
    throw new Error(`${testCase.description}: hostile answer does not exercise tool-use injection`);
  }
}

for (const testCase of cases) {
  const expectedAssertion = testCase.vars.expected_pass
    ? 'llm-rubric'
    : 'not-llm-rubric';
  if (JSON.stringify(testCase.providers) !== JSON.stringify(['frozen-human-answer'])) {
    throw new Error(`${testCase.description}: calibration does not target only echo`);
  }
  if (testCase.assert.length !== 3 || !testCase.assert.every((assertion) => assertion.type === expectedAssertion)) {
    throw new Error(
      `${testCase.description}: expected ${expectedAssertion}, got ${JSON.stringify(testCase.assert)}`,
    );
  }
  const providerLabels = testCase.assert.map((assertion) => assertion.provider).sort();
  const expectedProviderLabels = [
    'grader-gpt-5.6-luna-high',
    'grader-gpt-5.6-sol-high',
    'grader-gpt-5.6-terra-high',
  ];
  if (JSON.stringify(providerLabels) !== JSON.stringify(expectedProviderLabels)) {
    throw new Error(`${testCase.description}: unexpected graders ${JSON.stringify(providerLabels)}`);
  }
  const metrics = testCase.assert.map((assertion) => assertion.metric).sort();
  const expectedMetrics = [
    'agreement-gpt-5.6-luna',
    'agreement-gpt-5.6-sol',
    'agreement-gpt-5.6-terra',
  ];
  if (JSON.stringify(metrics) !== JSON.stringify(expectedMetrics)) {
    throw new Error(`${testCase.description}: unexpected metrics ${JSON.stringify(metrics)}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 grader calibration config echoes frozen answers through an isolated high-effort grader" {
  run node - "$ROOT" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const { parse } = require('yaml');

const root = process.argv[2];
const config = parse(
  fs.readFileSync(
    path.join(
      root,
      'evals/benchmarks/gpt-5.6-model-family/grader-promptfooconfig.yaml',
    ),
    'utf8',
  ),
);

if (JSON.stringify(config.prompts) !== JSON.stringify(['{{candidate_output}}'])) {
  throw new Error(`unexpected calibration prompt: ${JSON.stringify(config.prompts)}`);
}
const echo = config.providers.find((provider) => provider.id === 'echo');
if (!echo || echo.label !== 'frozen-human-answer') {
  throw new Error(`calibration target is not labelled echo: ${JSON.stringify(config.providers)}`);
}
if (config.tests !== 'file://grader-cases.cjs') {
  throw new Error(`calibration case loader is not config-relative: ${config.tests}`);
}

const graders = config.providers.filter(
  (provider) => provider.id === 'file://trace-enforced-codex-provider.mjs',
);
const models = graders.map((provider) => provider.config.model).sort();
if (JSON.stringify(models) !== JSON.stringify(['gpt-5.6-luna', 'gpt-5.6-sol', 'gpt-5.6-terra'])) {
  throw new Error(`unexpected calibration models: ${JSON.stringify(models)}`);
}
for (const grader of graders) {
  if (grader.label !== `grader-${grader.config.model}-high`) {
    throw new Error(`grader label does not identify model and effort: ${grader.label}`);
  }
  if (grader.config.model_reasoning_effort !== 'high') {
    throw new Error(`${grader.label}: grader effort is not high`);
  }
  if (!grader.config.cli_env.CODEX_HOME.includes('CODEX_EVAL_HOME_NO_PLUGINS')) {
    throw new Error(`${grader.label}: grader does not use the no-plugin home`);
  }
  if (grader.config.cli_config?.features?.plugins !== false) {
    throw new Error(`${grader.label}: grader does not disable plugin loading`);
  }
  if (!grader.config.working_dir.includes("default('../../../.dependencies/evals/agent-workspace')")) {
    throw new Error(`${grader.label}: grader workspace is not repository-root relative`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 execution isolation checker accepts direct model turns" {
  artifact="$(mktemp)"
  node - "$artifact" <<'NODE'
const fs = require('node:fs');
const file = process.argv[2];
fs.writeFileSync(file, JSON.stringify({
  results: {
    results: [{
      provider: { label: 'codex-gpt-5.6-terra-standard' },
      testCase: { vars: { case_id: 'fixture', sample_index: 1 } },
      response: {
        output: 'Direct answer',
        raw: JSON.stringify({
          finalResponse: 'Direct answer',
          items: [{ id: 'item_0', type: 'agent_message', text: 'Direct answer' }],
          notifications: [{
            method: 'rawResponseItem/completed',
            params: { item: { type: 'message', role: 'assistant', content: [] } },
          }],
          serverRequests: [],
        }),
      },
    }],
  },
}));
NODE

  run node "$ROOT/scripts/evals/check-gpt56-execution-isolation.mjs" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 0 ]
  [[ "$output" == *"verified 1 direct GPT-5.6 execution result"* ]]
}

@test "GPT-5.6 execution isolation checker rejects normalized and raw tool traces" {
  artifact="$(mktemp)"
  node - "$artifact" <<'NODE'
const fs = require('node:fs');
const file = process.argv[2];
const cleanMessage = {
  method: 'rawResponseItem/completed',
  params: { item: { type: 'message', role: 'assistant', content: [] } },
};
const result = (label, items, notifications, serverRequests = []) => ({
  provider: { label },
  testCase: { vars: { case_id: label, sample_index: 1 } },
  response: {
    output: 'Answer',
    raw: JSON.stringify({
      items,
      notifications,
      ...(serverRequests === null ? {} : { serverRequests }),
    }),
  },
});

fs.writeFileSync(file, JSON.stringify({
  results: {
    results: [
      result(
        'normalized-command',
        [{ type: 'command_execution', command: 'pwd' }],
        [cleanMessage],
      ),
      result(
        'raw-exec',
        [{ type: 'agent_message', text: 'Answer' }],
        [{
          method: 'rawResponseItem/completed',
          params: { item: { type: 'custom_tool_call', name: 'exec', input: 'return 1' } },
        }],
      ),
      result(
        'raw-wait',
        [{ type: 'agent_message', text: 'Answer' }],
        [{
          method: 'rawResponseItem/completed',
          params: { item: { type: 'function_call', name: 'wait', arguments: '{}' } },
        }],
      ),
      result(
        'plan-update',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage, { method: 'turn/plan/updated', params: { plan: [] } }],
      ),
      result(
        'missing-raw-events',
        [{ type: 'agent_message', text: 'Answer' }],
        [],
      ),
      result(
        'server-request',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage],
        [{ method: 'item/commandExecution/requestApproval', params: {} }],
      ),
      result(
        'missing-server-request-trace',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage],
        null,
      ),
      result(
        'started-command',
        [{ type: 'agent_message', text: 'Answer' }],
        [
          cleanMessage,
          {
            method: 'item/started',
            params: { item: { type: 'commandExecution', command: 'pwd' } },
          },
        ],
      ),
      result(
        'diff-update',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage, { method: 'turn/diff/updated', params: { diff: 'changed' } }],
      ),
      result(
        'plan-delta',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage, { method: 'item/plan/delta', params: { delta: 'Inspect' } }],
      ),
      result(
        'resolved-server-request',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage, { method: 'serverRequest/resolved', params: {} }],
      ),
      result(
        'unknown-notification',
        [{ type: 'agent_message', text: 'Answer' }],
        [cleanMessage, { method: 'future/toolActivity', params: {} }],
      ),
    ],
  },
}));
NODE

  run node "$ROOT/scripts/evals/check-gpt56-execution-isolation.mjs" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"normalized-command"*"disallowed normalized item command_execution"* ]]
  [[ "$output" == *"raw-exec"*"disallowed raw response item custom_tool_call"* ]]
  [[ "$output" == *"raw-wait"*"disallowed raw response item function_call"* ]]
  [[ "$output" == *"plan-update"*"plan-update activity"* ]]
  [[ "$output" == *"missing-raw-events"*"no verifiable raw response items"* ]]
  [[ "$output" == *"server-request"*"server request activity"* ]]
  [[ "$output" == *"missing-server-request-trace"*"no verifiable server request trace"* ]]
  [[ "$output" == *"started-command"*"tool notification item/started:commandExecution"* ]]
  [[ "$output" == *"diff-update"*"tool notification turn/diff/updated"* ]]
  [[ "$output" == *"plan-delta"*"tool notification item/plan/delta"* ]]
  [[ "$output" == *"resolved-server-request"*"tool notification serverRequest/resolved"* ]]
  [[ "$output" == *"unknown-notification"*"tool notification future/toolActivity"* ]]
}

@test "GPT-5.6 execution isolation checker rejects collaboration traces" {
  artifact="$(mktemp)"
  node - "$artifact" <<'NODE'
const fs = require('node:fs');
const file = process.argv[2];
fs.writeFileSync(file, JSON.stringify({
  results: {
    results: [{
      provider: { label: 'codex-gpt-5.6-luna-standard' },
      testCase: { vars: { case_id: 'fixture', sample_index: 1 } },
      response: {
        output: 'Delegated answer',
        raw: JSON.stringify({
          finalResponse: 'Delegated answer',
          items: [{
            id: 'item_0',
            type: 'collab_tool_call',
            tool: 'spawn_agent',
            status: 'completed',
          }],
        }),
      },
    }],
  },
}));
NODE

  run node "$ROOT/scripts/evals/check-gpt56-execution-isolation.mjs" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 1 ]
  [[ "$output" == *"collaboration or subagent activity"* ]]
  [[ "$output" == *"codex-gpt-5.6-luna-standard"* ]]
}

@test "trace-enforced Codex provider removes environments from every app-server turn" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const inner = {
  id: () => 'openai:codex-app-server',
  buildThreadStartParams: () => ({ model: 'gpt-5.6-terra' }),
  buildTurnStartParams: () => ({ threadId: 'thread-1' }),
  callApi: async () => {
    const requests = [
      inner.buildThreadStartParams({}),
      inner.buildTurnStartParams('thread-1', [], {}),
    ];
    if (!requests.every((request) =>
      Array.isArray(request.environments) && request.environments.length === 0
    )) {
      throw new Error(`environment-backed tools remain enabled: ${JSON.stringify(requests)}`);
    }
    return {
      output: 'Direct answer',
      raw: JSON.stringify({
        items: [{ type: 'agent_message', text: 'Direct answer' }],
        notifications: [
          {
            method: 'rawResponseItem/completed',
            params: { item: { type: 'message', role: 'assistant', content: [] } },
          },
        ],
        serverRequests: [],
      }),
    };
  },
};

let loadedProviderId;
let loadedContext;
const providerLoader = async (providerId, context) => {
  loadedProviderId = providerId;
  loadedContext = context;
  return inner;
};

const provider = new TraceEnforcedCodexProvider(
  {
    id: 'tool-free-fixture',
    config: {
      model: 'gpt-5.6-terra',
      working_dir: '/tmp/fixture',
      cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' },
    },
    env: { CODEX_HOME: '/tmp/codex-home-fixture' },
  },
  providerLoader,
);
const response = await provider.callApi('Answer directly.');

if (response.error) {
  throw new Error(response.error);
}
if (loadedProviderId !== 'openai:codex-app-server') {
  throw new Error(`unexpected inner provider: ${loadedProviderId}`);
}
const loadedOptions = loadedContext?.options;
if (
  loadedOptions?.id !== 'tool-free-fixture' ||
  loadedOptions?.config?.model !== 'gpt-5.6-terra' ||
  loadedOptions?.config?.working_dir !== '/tmp/fixture' ||
  loadedOptions?.env?.CODEX_HOME !== '/tmp/codex-home-fixture'
) {
  throw new Error(`inner provider options were not preserved: ${JSON.stringify(loadedContext)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider and isolation checker share the normal user-message lifecycle contract" {
  artifact="$(mktemp)"
  run node --input-type=module - "$ROOT" "$artifact" <<'NODE'
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const artifact = process.argv[3];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const submittedUserMessage = {
  id: 'item-1',
  type: 'userMessage',
  content: [{ type: 'text', text: 'Answer directly.' }],
};
const inner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async () => ({
    output: 'Direct answer',
    raw: JSON.stringify({
      items: [
        { id: 'item-1', type: 'user_message', text: 'Answer directly.' },
        { id: 'item-2', type: 'reasoning', text: 'Provide a direct answer.' },
        { id: 'item-3', type: 'agent_message', text: 'Direct answer' },
      ],
      notifications: [
        {
          method: 'item/started',
          params: { item: submittedUserMessage },
        },
        {
          method: 'item/completed',
          params: { item: submittedUserMessage },
        },
        {
          method: 'rawResponseItem/completed',
          params: {
            item: {
              type: 'message',
              role: 'assistant',
              content: [{ type: 'output_text', text: 'Direct answer' }],
            },
          },
        },
      ],
      serverRequests: [],
    }),
  }),
};
const provider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  async () => inner,
);
const response = await provider.callApi('Answer directly.');

if (response.error) {
  throw new Error(`normal user-message lifecycle was rejected: ${response.error}`);
}

fs.writeFileSync(artifact, JSON.stringify({
  results: {
    results: [{
      provider: { label: 'codex-gpt-5.6-terra-standard' },
      testCase: { vars: { case_id: 'normal-user-message-lifecycle', sample_index: 1 } },
      response,
    }],
  },
}));
NODE

  [ "$status" -eq 0 ]

  run node "$ROOT/scripts/evals/check-gpt56-execution-isolation.mjs" "$artifact"

  rm -f "$artifact"
  [ "$status" -eq 0 ]
  [[ "$output" == *"verified 1 direct GPT-5.6 execution result"* ]]
}

@test "trace-enforced Codex provider forces defense-in-depth app-server config" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

let loadedConfig;
let forwardedPromptConfig;
const inner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async (_prompt, context) => {
    forwardedPromptConfig = context?.prompt?.config;
    return {
      output: 'Direct answer',
      raw: JSON.stringify({
        items: [{ type: 'agent_message', text: 'Direct answer' }],
        notifications: [
          {
            method: 'rawResponseItem/completed',
            params: { item: { type: 'message', role: 'assistant', content: [] } },
          },
        ],
        serverRequests: [],
      }),
    };
  },
};
const provider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: {
      model: 'gpt-5.6-terra',
      sandbox_mode: 'danger-full-access',
      approval_policy: 'on-request',
      inherit_process_env: true,
      reuse_server: true,
      network_access_enabled: true,
      cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' },
      cli_config: {
        web_search: 'live',
        features: {
          plugins: true,
          shell_tool: true,
          multi_agent: true,
          browser_use: true,
        },
        tools: {
          experimental_request_user_input: { enabled: true },
        },
      },
    },
  },
  async (_providerId, context) => {
    loadedConfig = context.options.config;
    return inner;
  },
);
await provider.callApi('Answer directly.', {
  prompt: {
    config: {
      sandbox_mode: 'danger-full-access',
      approval_policy: 'on-request',
      reuse_server: true,
      cli_config: {
        web_search: 'live',
        features: { shell_tool: true, plugins: false },
      },
    },
  },
});

if (forwardedPromptConfig !== undefined) {
  throw new Error(
    `prompt-level isolation override was forwarded: ${JSON.stringify(forwardedPromptConfig)}`,
  );
}

if (loadedConfig.model !== 'gpt-5.6-terra') {
  throw new Error(`model was not preserved: ${JSON.stringify(loadedConfig)}`);
}
for (const [key, expected] of Object.entries({
  sandbox_mode: 'read-only',
  approval_policy: 'never',
  inherit_process_env: false,
  reuse_server: false,
  network_access_enabled: false,
  ephemeral: true,
  experimental_raw_events: true,
  include_raw_events: true,
})) {
  if (loadedConfig[key] !== expected) {
    throw new Error(`${key} was not forced to ${expected}: ${JSON.stringify(loadedConfig)}`);
  }
}
if (loadedConfig.cli_config?.web_search !== 'disabled') {
  throw new Error(`web search was not disabled: ${JSON.stringify(loadedConfig.cli_config)}`);
}
if (loadedConfig.cli_config?.features?.plugins !== true) {
  throw new Error('the explicit plugin-skills choice was not preserved');
}
const disabledFeatures = [
  'shell_tool',
  'unified_exec',
  'multi_agent',
  'enable_fanout',
  'apps',
  'enable_mcp_apps',
  'in_app_browser',
  'browser_use',
  'browser_use_full_cdp_access',
  'browser_use_external',
  'computer_use',
  'image_generation',
  'tool_suggest',
  'remote_plugin',
  'goals',
  'memories',
  'deferred_executor',
  'request_permissions_tool',
  'default_mode_request_user_input',
  'current_time_reminder',
  'skill_mcp_dependency_install',
  'hooks',
];
for (const feature of disabledFeatures) {
  if (loadedConfig.cli_config?.features?.[feature] !== false) {
    throw new Error(`${feature} was not disabled`);
  }
}
for (const nestedFeature of ['multi_agent_v2', 'token_budget', 'code_mode']) {
  if (loadedConfig.cli_config?.features?.[nestedFeature]?.enabled !== false) {
    throw new Error(`${nestedFeature}.enabled was not disabled`);
  }
}
if (loadedConfig.cli_config?.features?.code_mode_only !== false) {
  throw new Error('code_mode_only was not disabled');
}
if (
  loadedConfig.cli_config?.tools?.experimental_request_user_input?.enabled !==
  false
) {
  throw new Error('experimental request_user_input was not disabled');
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects observed tool-use traces" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const inner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async () => ({
    output: 'I used a tool.',
    raw: JSON.stringify({
      items: [
        { type: 'reasoning', text: 'I should inspect the workspace.' },
        { type: 'command_execution', command: 'pwd', exit_code: 0 },
        { type: 'agent_message', text: 'I used a tool.' },
      ],
    }),
  }),
};

const provider = new TraceEnforcedCodexProvider(
  {
    id: 'tool-free-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  async () => inner,
);
const response = await provider.callApi('Answer directly.');

if (!response.error?.includes('rejected command_execution')) {
  throw new Error(`tool-use trace was not rejected: ${JSON.stringify(response)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects successful responses without verifiable traces" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const fixtures = [
  { name: 'missing raw trace', response: { output: 'Direct answer' } },
  { name: 'invalid JSON trace', response: { output: 'Direct answer', raw: '{' } },
  {
    name: 'missing item list',
    response: { output: 'Direct answer', raw: JSON.stringify({ output: 'Direct answer' }) },
  },
  {
    name: 'non-array item list',
    response: { output: 'Direct answer', raw: JSON.stringify({ items: {} }) },
  },
];

for (const fixture of fixtures) {
  const inner = {
    buildThreadStartParams: () => ({}),
    buildTurnStartParams: () => ({}),
    callApi: async () => fixture.response,
  };
  const provider = new TraceEnforcedCodexProvider(
    {
      id: 'tool-free-fixture',
      config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
    },
    async () => inner,
  );

  let response;
  try {
    response = await provider.callApi('Answer directly.');
  } catch (error) {
    throw new Error(`${fixture.name} threw instead of returning a provider error: ${error}`);
  }
  if (!response.error?.includes('unverifiable trace')) {
    throw new Error(`${fixture.name} was not rejected: ${JSON.stringify(response)}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider forces raw events and rejects code-mode exec" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

let loadedContext;
const inner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async () => ({
    output: 'I used code mode.',
    raw: JSON.stringify({
      items: [{ type: 'agent_message', text: 'I used code mode.' }],
      notifications: [
        {
          method: 'rawResponseItem/completed',
          params: { item: { type: 'reasoning', summary: [] } },
        },
        {
          method: 'rawResponseItem/completed',
          params: {
            item: {
              type: 'custom_tool_call',
              name: 'exec',
              call_id: 'call-1',
              input: 'return await codex.tool("update_plan", {});',
            },
          },
        },
      ],
    }),
  }),
};

const provider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  async (_providerId, context) => {
    loadedContext = context;
    return inner;
  },
);
const response = await provider.callApi('Answer directly.');

if (
  loadedContext?.options?.config?.experimental_raw_events !== true ||
  loadedContext?.options?.config?.include_raw_events !== true
) {
  throw new Error(`raw event capture was not forced: ${JSON.stringify(loadedContext)}`);
}
if (!response.error?.includes('rejected raw custom_tool_call')) {
  throw new Error(`code-mode exec was not rejected: ${JSON.stringify(response)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider initializes one app server under concurrency" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

let releaseLoader;
const loaderGate = new Promise((resolve) => {
  releaseLoader = resolve;
});
let loaderCalls = 0;
let cleanupCalls = 0;
const providerLoader = async () => {
  loaderCalls += 1;
  await loaderGate;
  return {
    buildThreadStartParams: () => ({}),
    buildTurnStartParams: () => ({}),
    callApi: async () => ({
      output: 'Direct answer',
      raw: JSON.stringify({
        items: [{ type: 'agent_message', text: 'Direct answer' }],
        notifications: [
          {
            method: 'rawResponseItem/completed',
            params: { item: { type: 'message', role: 'assistant', content: [] } },
          },
        ],
        serverRequests: [],
      }),
    }),
    cleanup: async () => {
      cleanupCalls += 1;
    },
  };
};

const provider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  providerLoader,
);
const pendingCalls = [
  provider.callApi('First prompt.'),
  provider.callApi('Second prompt.'),
];
await new Promise((resolve) => setImmediate(resolve));
releaseLoader();
await Promise.all(pendingCalls);
await provider.cleanup();

if (loaderCalls !== 1) {
  throw new Error(`expected one app-server initialization, got ${loaderCalls}`);
}
if (cleanupCalls !== 1) {
  throw new Error(`expected one app-server cleanup, got ${cleanupCalls}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider requires a complete raw response-item trace" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const normalizedItems = [{ type: 'agent_message', text: 'Direct answer' }];
const fixtures = [
  {
    name: 'missing notifications',
    raw: { items: normalizedItems },
  },
  {
    name: 'empty notifications',
    raw: { items: normalizedItems, notifications: [] },
  },
  {
    name: 'raw completion without an item',
    raw: {
      items: normalizedItems,
      notifications: [{ method: 'rawResponseItem/completed', params: {} }],
    },
  },
];

for (const fixture of fixtures) {
  const inner = {
    buildThreadStartParams: () => ({}),
    buildTurnStartParams: () => ({}),
    callApi: async () => ({
      output: 'Direct answer',
      raw: JSON.stringify(fixture.raw),
    }),
  };
  const provider = new TraceEnforcedCodexProvider(
    {
      id: 'trace-enforced-fixture',
      config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
    },
    async () => inner,
  );
  const response = await provider.callApi('Answer directly.');
  if (!response.error?.includes('unverifiable raw response trace')) {
    throw new Error(`${fixture.name} was not rejected: ${JSON.stringify(response)}`);
  }
}

const cleanInner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async () => ({
    output: 'Direct answer',
    raw: JSON.stringify({
      items: normalizedItems,
      notifications: [
        {
          method: 'rawResponseItem/completed',
          params: { item: { type: 'reasoning', summary: [] } },
        },
        {
          method: 'rawResponseItem/completed',
          params: { item: { type: 'message', role: 'assistant', content: [] } },
        },
      ],
      serverRequests: [],
    }),
  }),
};
const cleanProvider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  async () => cleanInner,
);
const cleanResponse = await cleanProvider.callApi('Answer directly.');
if (cleanResponse.error) {
  throw new Error(`clean trace was rejected: ${cleanResponse.error}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects plan-update notifications" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const inner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async () => ({
    output: 'Direct answer',
    raw: JSON.stringify({
      items: [{ type: 'agent_message', text: 'Direct answer' }],
      notifications: [
        {
          method: 'rawResponseItem/completed',
          params: { item: { type: 'message', role: 'assistant', content: [] } },
        },
        {
          method: 'turn/plan/updated',
          params: { turnId: 'turn-1', plan: [{ step: 'Inspect files' }] },
        },
      ],
    }),
  }),
};
const provider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  async () => inner,
);
const response = await provider.callApi('Answer directly.');

if (!response.error?.includes('rejected turn/plan/updated')) {
  throw new Error(`plan activity was not rejected: ${JSON.stringify(response)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider requires an empty server-request trace" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);
const baseRaw = {
  items: [{ type: 'agent_message', text: 'Direct answer' }],
  notifications: [
    {
      method: 'rawResponseItem/completed',
      params: { item: { type: 'message', role: 'assistant', content: [] } },
    },
  ],
};
const fixtures = [
  { name: 'missing server-request trace', raw: baseRaw },
  {
    name: 'command approval request',
    raw: {
      ...baseRaw,
      serverRequests: [
        { method: 'item/commandExecution/requestApproval', params: {} },
      ],
    },
  },
];

for (const fixture of fixtures) {
  const inner = {
    buildThreadStartParams: () => ({}),
    buildTurnStartParams: () => ({}),
    callApi: async () => ({
      output: 'Direct answer',
      raw: JSON.stringify(fixture.raw),
    }),
  };
  const provider = new TraceEnforcedCodexProvider(
    {
      id: 'trace-enforced-fixture',
      config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
    },
    async () => inner,
  );
  const response = await provider.callApi('Answer directly.');
  if (!response.error?.includes('server request trace')) {
    throw new Error(`${fixture.name} was not rejected: ${JSON.stringify(response)}`);
  }
}

const cleanInner = {
  buildThreadStartParams: () => ({}),
  buildTurnStartParams: () => ({}),
  callApi: async () => ({
    output: 'Direct answer',
    raw: JSON.stringify({ ...baseRaw, serverRequests: [] }),
  }),
};
const cleanProvider = new TraceEnforcedCodexProvider(
  {
    id: 'trace-enforced-fixture',
    config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
  },
  async () => cleanInner,
);
const cleanResponse = await cleanProvider.callApi('Answer directly.');
if (cleanResponse.error) {
  throw new Error(`empty server-request trace was rejected: ${cleanResponse.error}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects incomplete tool notifications" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);
const cleanRaw = {
  items: [{ type: 'agent_message', text: 'Direct answer' }],
  notifications: [
    {
      method: 'rawResponseItem/completed',
      params: { item: { type: 'message', role: 'assistant', content: [] } },
    },
  ],
  serverRequests: [],
};
const fixtures = [
  {
    name: 'started command',
    notification: {
      method: 'item/started',
      params: { item: { type: 'commandExecution', command: 'pwd' } },
    },
    expected: 'item/started:commandExecution',
  },
  {
    name: 'diff update',
    notification: {
      method: 'turn/diff/updated',
      params: { diff: 'changed' },
    },
    expected: 'turn/diff/updated',
  },
  {
    name: 'plan delta',
    notification: {
      method: 'item/plan/delta',
      params: { delta: 'Inspect files' },
    },
    expected: 'item/plan/delta',
  },
  {
    name: 'resolved server request',
    notification: {
      method: 'serverRequest/resolved',
      params: { requestId: 'request-1' },
    },
    expected: 'serverRequest/resolved',
  },
  {
    name: 'unknown notification',
    notification: {
      method: 'future/toolActivity',
      params: {},
    },
    expected: 'future/toolActivity',
  },
];

for (const fixture of fixtures) {
  const inner = {
    buildThreadStartParams: () => ({}),
    buildTurnStartParams: () => ({}),
    callApi: async () => ({
      output: 'Direct answer',
      raw: JSON.stringify({
        ...cleanRaw,
        notifications: [...cleanRaw.notifications, fixture.notification],
      }),
    }),
  };
  const provider = new TraceEnforcedCodexProvider(
    {
      id: 'trace-enforced-fixture',
      config: { cli_env: { CODEX_HOME: '/tmp/codex-home-fixture' } },
    },
    async () => inner,
  );
  const response = await provider.callApi('Answer directly.');
  if (!response.error?.includes(`rejected ${fixture.expected}`)) {
    throw new Error(`${fixture.name} was not rejected: ${JSON.stringify(response)}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}
