#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  FIXTURE_TMP=""
  TMP_REPO=""
  NO_PLUGINS_HOME=""
  TARGETED_HOME=""
  EVAL_BACKUP_DIR=""
}

teardown() {
  [ -z "$FIXTURE_TMP" ] || rm -rf "$FIXTURE_TMP"
  [ -z "$TMP_REPO" ] || rm -rf "$TMP_REPO"
  [ -z "$NO_PLUGINS_HOME" ] || rm -rf "$NO_PLUGINS_HOME"
  [ -z "$TARGETED_HOME" ] || rm -rf "$TARGETED_HOME"

  if [ -n "$EVAL_BACKUP_DIR" ]; then
    rm -f "$ROOT/evals/out/results.json" "$ROOT/evals/out/status.json"
    [ ! -f "$EVAL_BACKUP_DIR/results.json" ] || cp "$EVAL_BACKUP_DIR/results.json" "$ROOT/evals/out/results.json"
    [ ! -f "$EVAL_BACKUP_DIR/status.json" ] || cp "$EVAL_BACKUP_DIR/status.json" "$ROOT/evals/out/status.json"
    rm -rf "$EVAL_BACKUP_DIR"
  fi
}

@test "behavior loader reads recursive full-marketplace fixtures with coverage metadata" {
  run node - <<NODE
const generateTests = require('$ROOT/evals/promptfoo/load-harness-cases.cjs');
const tests = generateTests();
const failures = [];
const caseIds = new Set(tests.map((testCase) => testCase.vars?.case_id));

for (const required of [
  'worktrees-setup-natural-trigger',
  'babysit-pr-natural-trigger',
  'engineering-scaffold-natural-trigger',
  'agentic-scaffold-evals-natural-trigger',
]) {
  if (!caseIds.has(required)) failures.push(`missing ${required}`);
}

for (const testCase of tests) {
  const vars = testCase.vars || {};
  if (!vars.fixture_file || !vars.fixture_file.includes('evals/fixtures/behavior/')) {
    failures.push(`${vars.case_id}: missing recursive fixture_file`);
  }
  if (!Array.isArray(vars.coverage_kinds) || vars.coverage_kinds.length === 0) {
    failures.push(`${vars.case_id}: missing coverage_kinds`);
  }
  if (!['safety-critical', 'standard'].includes(vars.value_gate_mode)) {
    failures.push(`${vars.case_id}: invalid value_gate_mode`);
  }
  if ('plugin_mode' in vars || 'pluginMode' in vars) {
    failures.push(`${vars.case_id}: plugin mode should be inferred from provider label`);
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "one behavior fixture preserves list metadata in one target request" {
  FIXTURE_TMP="$(mktemp -d)"
  mkdir -p "$FIXTURE_TMP/evals/fixtures/behavior" "$FIXTURE_TMP/evals"
  call_log="$FIXTURE_TMP/target-calls.jsonl"
  config="$FIXTURE_TMP/promptfooconfig.yaml"

  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {
    "case_id": "atomic-list-metadata",
    "behavior": "preserve list metadata",
    "prompt": "Return ok.",
    "plugins": ["alpha", "beta"],
    "skills": ["one", "two"],
    "coverage": {"kinds": ["natural-trigger", "hard-guard"]},
    "minPassRate": 1,
    "semanticRubric": "Pass if the output is ok.",
    "hardAssertions": []
  }
]
JSON
  cat >"$FIXTURE_TMP/evals/matrix.json" <<'JSON'
{"valueGates":{"defaultBaselineLiftThreshold":0.1}}
JSON
  cat >"$FIXTURE_TMP/target-provider.cjs" <<'JS'
const fs = require("node:fs");

module.exports = class TargetProvider {
  id() {
    return "target";
  }

  async callApi(_prompt, context) {
    fs.appendFileSync(
      process.env.TARGET_CALL_LOG,
      `${JSON.stringify({
        plugins: context.vars.plugins,
        skills: context.vars.skills,
        coverage_kinds: context.vars.coverage_kinds,
      })}\n`,
    );
    return { output: "ok" };
  }
};
JS
  cat >"$FIXTURE_TMP/grader-provider.cjs" <<'JS'
module.exports = class GraderProvider {
  id() {
    return "grader";
  }

  async callApi() {
    return {
      output: JSON.stringify({ pass: true, score: 1, reason: "ok" }),
    };
  }
};
JS
  cat >"$config" <<YAML
prompts:
  - "{{scenario_prompt}}"
providers:
  - id: file://./target-provider.cjs
tests: file://$ROOT/evals/promptfoo/load-harness-cases.cjs
defaultTest:
  options:
    provider:
      text:
        id: file://./grader-provider.cjs
commandLineOptions:
  cache: false
  share: false
  write: true
YAML

  run bash -c '
    cd "$1"
    TARGET_CALL_LOG="$2" \
      PROMPTFOO_DISABLE_VAR_EXPANSION=false \
      PROMPTFOO_DISABLE_TELEMETRY=1 \
      PROMPTFOO_CONFIG_DIR="$1/.promptfoo" \
      "$3" eval -c "$4" --no-cache --no-share -o "$1/results.json"
  ' _ "$FIXTURE_TMP" "$call_log" "$ROOT/node_modules/.bin/promptfoo" "$config"

  [ "$status" -eq 0 ]
  call_count="$(wc -l <"$call_log")"
  printf 'target_call_count=%s\n' "$call_count" >&3
  [ "$call_count" -eq 1 ]
  run jq -e -s '
    . == [{
      plugins: ["alpha", "beta"],
      skills: ["one", "two"],
      coverage_kinds: ["natural-trigger", "hard-guard"]
    }]
  ' "$call_log"
  [ "$status" -eq 0 ]
  run jq -e '
    .results.results as $results
    | ($results | length) == 1
      and $results[0].testCase.vars.plugins == ["alpha", "beta"]
      and $results[0].testCase.vars.skills == ["one", "two"]
      and $results[0].testCase.vars.coverage_kinds == ["natural-trigger", "hard-guard"]
  ' "$FIXTURE_TMP/results.json"
  [ "$status" -eq 0 ]
}

@test "behavior fixture loader rejects duplicate case ids across recursive files" {
  FIXTURE_TMP="$(mktemp -d)"
  mkdir -p "$FIXTURE_TMP/evals/fixtures/behavior/one" "$FIXTURE_TMP/evals/fixtures/behavior/two"
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/one/cases.json" <<'JSON'
[{"case_id":"duplicate","behavior":"one","prompt":"one"}]
JSON
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/two/cases.json" <<'JSON'
[{"case_id":"duplicate","behavior":"two","prompt":"two"}]
JSON

  run node - "$FIXTURE_TMP" <<NODE
const { loadBehaviorCases } = require('$ROOT/evals/promptfoo/fixtures.cjs');
loadBehaviorCases({ root: process.argv[2] });
NODE

  [ "$status" -ne 0 ]
  [[ "$output" == *'Duplicate case_id "duplicate"'* ]]
}

@test "selected behavior plugins are deduplicated in canonical order" {
  FIXTURE_TMP="$(mktemp -d)"
  mkdir -p "$FIXTURE_TMP/evals/fixtures/behavior"
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {"case_id":"selected-one","plugins":["zeta-plugin","alpha-plugin"]},
  {"case_id":"selected-two","plugins":["middle-plugin","alpha-plugin"]},
  {"case_id":"excluded","plugins":["excluded-plugin"]}
]
JSON

  run node - "$ROOT" "$FIXTURE_TMP" <<'NODE'
const path = require('node:path');
const root = process.argv[2];
const fixtureRoot = process.argv[3];
const { selectedBehaviorPluginNames } = require(path.join(root, 'evals/promptfoo/fixtures.cjs'));
const actual = selectedBehaviorPluginNames({ root: fixtureRoot, caseFilter: 'selected-' });
const expected = ['alpha-plugin', 'middle-plugin', 'zeta-plugin'];

if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  throw new Error(`${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "coverage checker requires every skill to have exhaustive behavior coverage or an explicit decision" {
  run node "$ROOT/scripts/evals/check-coverage.mjs"

  [ "$status" -eq 0 ]
  [[ "$output" == *"coverage complete"* ]]

  FIXTURE_TMP="$(mktemp -d)"
  fixture="$FIXTURE_TMP"
  mkdir -p "$fixture/plugins/example/skills/alpha" "$fixture/evals/fixtures/behavior/example"
  mkdir -p "$fixture/.agents/plugins" "$fixture/.claude-plugin" "$fixture/plugins/example/.codex-plugin" "$fixture/plugins/example/.claude-plugin"
  cat >"$fixture/.agents/plugins/marketplace.json" <<'JSON'
{"plugins":[{"name":"example","source":{"source":"local","path":"./plugins/example"}}]}
JSON
  cat >"$fixture/.claude-plugin/marketplace.json" <<'JSON'
{"plugins":[{"name":"example","source":"./plugins/example","version":"0.1.0"}]}
JSON
  cat >"$fixture/plugins/example/.codex-plugin/plugin.json" <<'JSON'
{"name":"example","version":"0.1.0"}
JSON
  cat >"$fixture/plugins/example/.claude-plugin/plugin.json" <<'JSON'
{"name":"example","version":"0.1.0"}
JSON
  cat >"$fixture/plugins/example/skills/alpha/SKILL.md" <<'MD'
---
name: alpha
description: Example skill
---
MD
  cat >"$fixture/evals/fixtures/behavior/example/cases.json" <<'JSON'
[
  {
    "case_id": "alpha-trigger",
    "behavior": "trigger only",
    "prompt": "Use alpha",
    "plugins": ["example"],
    "skills": ["alpha"],
    "coverage": {"kinds": ["natural-trigger"]},
    "minPassRate": 1,
    "semanticRubric": "Pass if alpha is used.",
    "hardAssertions": [],
    "calibration": {"pass": ["ok"], "fail": ["bad"]}
  }
]
JSON

  run node "$ROOT/scripts/evals/check-coverage.mjs" --root "$fixture"

  [ "$status" -ne 0 ]
  [[ "$output" == *"example:alpha"* ]]
  [[ "$output" == *"missing coverage kinds"* ]]
}

@test "coverage checker accepts explicit decisions outside behavior fixtures" {
  FIXTURE_TMP="$(mktemp -d)"
  fixture="$FIXTURE_TMP"
  mkdir -p "$fixture/plugins/example/skills/alpha" "$fixture/evals/fixtures"
  mkdir -p "$fixture/.agents/plugins" "$fixture/.claude-plugin" "$fixture/plugins/example/.codex-plugin"
  cat >"$fixture/.agents/plugins/marketplace.json" <<'JSON'
{"plugins":[{"name":"example","source":{"source":"local","path":"./plugins/example"}}]}
JSON
  cat >"$fixture/.claude-plugin/marketplace.json" <<'JSON'
{"plugins":[]}
JSON
  cat >"$fixture/plugins/example/.codex-plugin/plugin.json" <<'JSON'
{"name":"example","version":"0.1.0"}
JSON
  cat >"$fixture/plugins/example/skills/alpha/SKILL.md" <<'MD'
---
name: alpha
description: Example skill
---
MD
  cat >"$fixture/evals/fixtures/coverage-decisions.json" <<'JSON'
[
  {
    "plugin": "example",
    "skill": "alpha",
    "decision": "deferred",
    "reason": "Codex-only behavior coverage is deferred until the harness supports provider-scoped behavior cases without running the same case in Claude Code."
  }
]
JSON

  run node "$ROOT/scripts/evals/check-coverage.mjs" --root "$fixture"

  [ "$status" -eq 0 ]
  [[ "$output" == *"coverage complete"* ]]
}

@test "generated behavior config expands provider variants across plugin modes" {
  run node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: claude-code-sonnet-full-marketplace"* ]]
  [[ "$output" == *"label: claude-code-sonnet-targeted-plugins"* ]]
  [[ "$output" == *"label: claude-code-sonnet-no-plugins"* ]]
  [[ "$output" == *"label: codex-gpt-5.6-terra-full-marketplace"* ]]
  [[ "$output" == *"label: codex-gpt-5.6-terra-targeted-plugins"* ]]
  [[ "$output" == *"label: codex-gpt-5.6-terra-no-plugins"* ]]
  [[ "$output" == *"pluginMode: no-plugins"* ]]
  [[ "$output" == *"pluginMode: targeted-plugins"* ]]
  [[ "$output" == *"pluginMode: full-marketplace"* ]]
  [[ "$output" == *"load-harness-cases.cjs?pluginMode={{ provider.pluginMode }}"* ]]
}

@test "filtered behavior config gives Claude targeted provider exactly the selected case plugins" {
  run env EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture node - "$ROOT" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = process.argv[2];
const result = spawnSync(
  process.execPath,
  [path.join(root, 'scripts/evals/generate-config.mjs'), '--suite', 'behavior', '--stdout'],
  { cwd: root, encoding: 'utf8', env: process.env },
);
if (result.status !== 0) {
  process.stderr.write(result.stderr || result.stdout);
  process.exit(result.status);
}

function providerPluginPaths(label) {
  const marker = `    label: ${label}\n`;
  const start = result.stdout.indexOf(marker);
  if (start < 0) throw new Error(`missing provider ${label}`);
  const next = result.stdout.indexOf('\n  - id:', start + marker.length);
  const section = result.stdout.slice(start, next < 0 ? undefined : next);
  return [...section.matchAll(/^\s+path: "([^"]+)"$/gm)]
    .map((match) => match[1])
    .sort();
}

const targeted = providerPluginPaths('claude-code-sonnet-targeted-plugins');
const full = providerPluginPaths('claude-code-sonnet-full-marketplace');
const expectedTargeted = [path.join(root, 'plugins/tiber')];
const expectedFull = JSON.parse(
  fs.readFileSync(path.join(root, '.claude-plugin/marketplace.json'), 'utf8'),
).plugins.map(({ name }) => path.join(root, 'plugins', name)).sort();

if (JSON.stringify(targeted) !== JSON.stringify(expectedTargeted)) {
  throw new Error(`targeted provider paths ${JSON.stringify(targeted)} != ${JSON.stringify(expectedTargeted)}`);
}
if (JSON.stringify(full) !== JSON.stringify(expectedFull)) {
  throw new Error(`full provider paths ${JSON.stringify(full)} != ${JSON.stringify(expectedFull)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "generated metadata records exact filtered provider plugin compositions" {
  FIXTURE_TMP="$(mktemp -d)"
  generated_config="$FIXTURE_TMP/config.yaml"
  generated_metadata="$FIXTURE_TMP/metadata.json"

  run env EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture node \
    "$ROOT/scripts/evals/generate-config.mjs" \
    --suite behavior \
    --output "$generated_config" \
    --metadata-output "$generated_metadata"

  [ "$status" -eq 0 ]
  jq -e '
    def plugins($label):
      [.providerCompositions[] | select(.label == $label) | .plugins] | first;
    plugins("claude-code-sonnet-targeted-plugins") == ["tiber"]
      and plugins("codex-gpt-5.6-terra-targeted-plugins") == ["tiber"]
      and plugins("claude-code-sonnet-no-plugins") == []
      and plugins("codex-gpt-5.6-terra-no-plugins") == []
      and (plugins("claude-code-sonnet-full-marketplace") | index("advisor") | not)
      and (plugins("codex-gpt-5.6-terra-full-marketplace") | index("advisor") != null)
  ' "$generated_metadata"
  run node - "$generated_config" "$generated_metadata" <<'NODE'
const fs = require('node:fs');
const YAML = require('yaml');
const config = YAML.parse(fs.readFileSync(process.argv[2], 'utf8'));
const metadata = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));

if (JSON.stringify(config.metadata.providerCompositions) !== JSON.stringify(metadata.providerCompositions)) {
  throw new Error('YAML and sidecar provider compositions differ');
}
NODE
  [ "$status" -eq 0 ]
}

@test "codex eval home preparation supports no-plugin and targeted-plugin modes" {
  NO_PLUGINS_HOME="$(mktemp -d)"
  TARGETED_HOME="$(mktemp -d)"
  no_plugins_home="$NO_PLUGINS_HOME"
  targeted_home="$TARGETED_HOME"

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$no_plugins_home" --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  [ -f "$no_plugins_home/config.toml" ]
  ! grep -q '\[plugins\."' "$no_plugins_home/config.toml"
  [ ! -d "$no_plugins_home/plugins/cache/ai-plugins/agentic-systems-engineering" ]

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$targeted_home" --plugin-mode targeted-plugins --plugins worktrees,engineering-standards

  [ "$status" -eq 0 ]
  grep -q '\[plugins\."worktrees@ai-plugins"\]' "$targeted_home/config.toml"
  grep -q '\[plugins\."engineering-standards@ai-plugins"\]' "$targeted_home/config.toml"
  ! grep -q '\[plugins\."babysit-pr@ai-plugins"\]' "$targeted_home/config.toml"
  [ -d "$targeted_home/plugins/cache/ai-plugins/worktrees" ]
  [ ! -d "$targeted_home/plugins/cache/ai-plugins/babysit-pr" ]

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$targeted_home" --plugin-mode targeted-plugins --plugins missing-plugin

  [ "$status" -ne 0 ]
  [[ "$output" == *"unknown targeted plugin(s): missing-plugin"* ]]
}

@test "explicitly empty targeted and skills-only lists fail before replacing an eval home" {
  FIXTURE_TMP="$(mktemp -d)"

  for plugin_mode in targeted-plugins skills-only-marketplace; do
    eval_home="$FIXTURE_TMP/$plugin_mode"
    mkdir -p "$eval_home"
    printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
    printf 'preserve me\n' >"$eval_home/sentinel"

    run env OPENAI_API_KEY=fixture node \
      "$ROOT/scripts/evals/prepare-codex-home.mjs" \
      "$eval_home" \
      --plugin-mode "$plugin_mode" \
      --plugins ""

    [ "$status" -ne 0 ]
    [[ "$output" == *"$plugin_mode mode requires a non-empty --plugins list"* ]]
    [ -f "$eval_home/sentinel" ]
    [ ! -f "$eval_home/config.toml" ]
  done
}

@test "omitted skills-only list retains full marketplace behavior" {
  FIXTURE_TMP="$(mktemp -d)"
  eval_home="$FIXTURE_TMP/skills-only"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode skills-only-marketplace

  [ "$status" -eq 0 ]
  expected_count="$(jq '.plugins | length' "$ROOT/.agents/plugins/marketplace.json")"
  [ "$(grep -c '^\[plugins\.' "$eval_home/config.toml")" -eq "$expected_count" ]

  while IFS= read -r plugin; do
    grep -q "\\[plugins\\.\"${plugin}@ai-plugins\"\\]" "$eval_home/config.toml"
    [ -d "$eval_home/plugins/cache/ai-plugins/$plugin" ]
  done < <(jq -r '.plugins[].name' "$ROOT/.agents/plugins/marketplace.json")

  agentic_version="$(jq -r '.version' "$ROOT/plugins/agentic-systems-engineering/.codex-plugin/plugin.json")"
  agentic_cache="$eval_home/plugins/cache/ai-plugins/agentic-systems-engineering/$agentic_version"
  [ -d "$agentic_cache/skills" ]
  [ ! -e "$agentic_cache/bin" ]
  [ ! -e "$agentic_cache/.mcp.json" ]
}

@test "improvement loop scope guards reject edits outside their allowed surfaces" {
  TMP_REPO="$(mktemp -d)"
  tmp_repo="$TMP_REPO"
  mkdir -p "$tmp_repo/plugins/example/skills/alpha" "$tmp_repo/evals/fixtures/behavior"
  cd "$tmp_repo"
  git init -q
  git config user.email test@example.com
  git config user.name Test
  git config commit.gpgsign false
  echo 'skill' > plugins/example/skills/alpha/SKILL.md
  echo 'fixture' > evals/fixtures/behavior/cases.json
  git add .
  git commit -q -m initial

  echo 'changed' > plugins/example/skills/alpha/SKILL.md
  run "$ROOT/scripts/evals/check-loop-scope.sh" improve-plugins
  [ "$status" -eq 0 ]

  echo 'changed' > evals/fixtures/behavior/cases.json
  run "$ROOT/scripts/evals/check-loop-scope.sh" improve-plugins
  [ "$status" -ne 0 ]
  [[ "$output" == *"disallowed path for improve-plugins"* ]]

  git checkout -q -- .
  echo 'changed' > evals/fixtures/behavior/cases.json
  run "$ROOT/scripts/evals/check-loop-scope.sh" improve-evals
  [ "$status" -eq 0 ]

  echo 'changed' > plugins/example/skills/alpha/SKILL.md
  run "$ROOT/scripts/evals/check-loop-scope.sh" improve-evals
  [ "$status" -ne 0 ]
  [[ "$output" == *"disallowed path for improve-evals"* ]]

  git checkout -q -- .
  mkdir -p docs
  echo 'new' > docs/new-file.md
  run "$ROOT/scripts/evals/check-loop-scope.sh" improve-plugins
  [ "$status" -ne 0 ]
  [[ "$output" == *"disallowed path for improve-plugins: docs/new-file.md"* ]]
}

@test "dashboard aggregates provider variant plugin mode and value gates" {
  EVAL_BACKUP_DIR="$(mktemp -d)"
  [ ! -f "$ROOT/evals/out/results.json" ] || cp "$ROOT/evals/out/results.json" "$EVAL_BACKUP_DIR/results.json"
  [ ! -f "$ROOT/evals/out/status.json" ] || cp "$ROOT/evals/out/status.json" "$EVAL_BACKUP_DIR/status.json"
  mkdir -p "$ROOT/evals/out"
  cat >"$ROOT/evals/out/results.json" <<'JSON'
{
  "results": [
    {
      "provider": {"label": "codex-gpt-5.6-terra-full-marketplace"},
      "testCase": {"vars": {"case_id": "alpha", "behavior": "Alpha", "provider_variant": "codex-gpt-5.6-terra", "plugin_mode": "full-marketplace", "plugins": ["example"], "skills": ["alpha"], "min_pass_rate": 0.8, "value_gate_mode": "standard", "baseline_lift_threshold": 0.1, "hard_guard_status": "passed"}},
      "gradingResult": {"pass": true, "score": 1}
    },
    {
      "provider": {"label": "codex-gpt-5.6-terra-no-plugins"},
      "testCase": {"vars": {"case_id": "alpha", "behavior": "Alpha", "provider_variant": "codex-gpt-5.6-terra", "plugin_mode": "no-plugins", "plugins": ["example"], "skills": ["alpha"], "min_pass_rate": 0.8, "value_gate_mode": "standard", "baseline_lift_threshold": 0.1, "hard_guard_status": "passed"}},
      "gradingResult": {"pass": false, "score": 0}
    }
  ]
}
JSON

  run node "$ROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  run node - <<NODE
const fs = require('fs');
const summary = JSON.parse(fs.readFileSync('$ROOT/site/evals/summary.json', 'utf8'));
if (!summary.aggregates.some((group) => group.providerVariant === 'codex-gpt-5.6-terra' && group.pluginMode === 'full-marketplace')) {
  throw new Error('missing provider variant/plugin mode aggregate');
}
if (!summary.valueGateSummaries.some((gate) => gate.caseId === 'alpha' && gate.providerVariant === 'codex-gpt-5.6-terra' && gate.status === 'pass')) {
  throw new Error(`missing passing value gate: ${JSON.stringify(summary.valueGateSummaries)}`);
}
NODE
  [ "$status" -eq 0 ]
}

@test "dashboard treats blocked baselines as unsupported value gates" {
  EVAL_BACKUP_DIR="$(mktemp -d)"
  [ ! -f "$ROOT/evals/out/results.json" ] || cp "$ROOT/evals/out/results.json" "$EVAL_BACKUP_DIR/results.json"
  [ ! -f "$ROOT/evals/out/status.json" ] || cp "$ROOT/evals/out/status.json" "$EVAL_BACKUP_DIR/status.json"
  mkdir -p "$ROOT/evals/out"
  cat >"$ROOT/evals/out/results.json" <<'JSON'
{
  "results": [
    {
      "provider": {"label": "codex-gpt-5.6-terra-full-marketplace"},
      "testCase": {"vars": {"case_id": "blocked-baseline", "behavior": "Blocked", "provider_variant": "codex-gpt-5.6-terra", "plugin_mode": "full-marketplace", "plugins": ["example"], "skills": ["alpha"], "min_pass_rate": 0.8, "value_gate_mode": "standard", "baseline_lift_threshold": 0.1}},
      "gradingResult": {"pass": true, "score": 1}
    },
    {
      "provider": {"label": "codex-gpt-5.6-terra-no-plugins"},
      "testCase": {"vars": {"case_id": "blocked-baseline", "behavior": "Blocked", "provider_variant": "codex-gpt-5.6-terra", "plugin_mode": "no-plugins", "plugins": ["example"], "skills": ["alpha"], "min_pass_rate": 0.8, "value_gate_mode": "standard", "baseline_lift_threshold": 0.1}},
      "gradingResult": {"pass": false, "score": 0, "reason": "provider unavailable: not logged in"}
    }
  ]
}
JSON

  run node "$ROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  run node - <<NODE
const fs = require('fs');
const summary = JSON.parse(fs.readFileSync('$ROOT/site/evals/summary.json', 'utf8'));
const gate = summary.valueGateSummaries.find((item) => item.caseId === 'blocked-baseline');
if (!gate || gate.status !== 'unsupported') {
  throw new Error(JSON.stringify(summary.valueGateSummaries));
}
NODE
  [ "$status" -eq 0 ]
}
