#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "behavior loader reads recursive full-marketplace fixtures with coverage metadata" {
  run node - <<'NODE'
const generateTests = require('./evals/promptfoo/load-harness-cases.cjs');
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
  if (vars.plugin_mode !== 'full-marketplace') {
    failures.push(`${vars.case_id}: default loader should emit full-marketplace mode`);
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "coverage checker requires every skill to have exhaustive behavior coverage or an explicit decision" {
  run node "$ROOT/scripts/evals/check-coverage.mjs"

  [ "$status" -eq 0 ]
  [[ "$output" == *"coverage complete"* ]]

  fixture="$(mktemp -d)"
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

@test "generated behavior config expands provider variants across plugin modes" {
  run node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: claude-code-sonnet-full-marketplace"* ]]
  [[ "$output" == *"label: claude-code-sonnet-targeted-plugins"* ]]
  [[ "$output" == *"label: claude-code-sonnet-no-plugins"* ]]
  [[ "$output" == *"label: codex-gpt-5.5-full-marketplace"* ]]
  [[ "$output" == *"label: codex-gpt-5.5-targeted-plugins"* ]]
  [[ "$output" == *"label: codex-gpt-5.5-no-plugins"* ]]
  [[ "$output" == *"pluginMode: no-plugins"* ]]
  [[ "$output" == *"pluginMode: targeted-plugins"* ]]
  [[ "$output" == *"pluginMode: full-marketplace"* ]]
  [[ "$output" == *"load-harness-cases.cjs?pluginMode={{ provider.pluginMode }}"* ]]
}

@test "codex eval home preparation supports no-plugin and targeted-plugin modes" {
  no_plugins_home="$(mktemp -d)"
  targeted_home="$(mktemp -d)"

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$no_plugins_home" --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  ! grep -q '\[plugins\."' "$no_plugins_home/config.toml"
  [ ! -d "$no_plugins_home/plugins/cache/ai-plugins/agentic-systems-engineering" ]

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$targeted_home" --plugin-mode targeted-plugins --plugins worktrees,engineering-standards

  [ "$status" -eq 0 ]
  grep -q '\[plugins\."worktrees@ai-plugins"\]' "$targeted_home/config.toml"
  grep -q '\[plugins\."engineering-standards@ai-plugins"\]' "$targeted_home/config.toml"
  ! grep -q '\[plugins\."babysit-pr@ai-plugins"\]' "$targeted_home/config.toml"
  [ -d "$targeted_home/plugins/cache/ai-plugins/worktrees" ]
  [ ! -d "$targeted_home/plugins/cache/ai-plugins/babysit-pr" ]

  rm -rf "$no_plugins_home" "$targeted_home"
}

@test "improvement loop scope guards reject edits outside their allowed surfaces" {
  tmp_repo="$(mktemp -d)"
  mkdir -p "$tmp_repo/plugins/example/skills/alpha" "$tmp_repo/evals/fixtures/behavior"
  cd "$tmp_repo"
  git init -q
  git config user.email test@example.com
  git config user.name Test
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

  rm -rf "$tmp_repo"
}

@test "dashboard aggregates provider variant plugin mode and value gates" {
  backup_dir="$(mktemp -d)"
  mkdir -p "$backup_dir"
  [ ! -f "$ROOT/evals/out/results.json" ] || cp "$ROOT/evals/out/results.json" "$backup_dir/results.json"
  [ ! -f "$ROOT/evals/out/status.json" ] || cp "$ROOT/evals/out/status.json" "$backup_dir/status.json"
  mkdir -p "$ROOT/evals/out"
  cat >"$ROOT/evals/out/results.json" <<'JSON'
{
  "results": [
    {
      "provider": {"label": "codex-gpt-5.5-full-marketplace"},
      "testCase": {"vars": {"case_id": "alpha", "behavior": "Alpha", "provider_variant": "codex-gpt-5.5", "plugin_mode": "full-marketplace", "plugins": ["example"], "skills": ["alpha"], "min_pass_rate": 0.8, "value_gate_mode": "standard", "baseline_lift_threshold": 0.1, "hard_guard_status": "passed"}},
      "gradingResult": {"pass": true, "score": 1}
    },
    {
      "provider": {"label": "codex-gpt-5.5-no-plugins"},
      "testCase": {"vars": {"case_id": "alpha", "behavior": "Alpha", "provider_variant": "codex-gpt-5.5", "plugin_mode": "no-plugins", "plugins": ["example"], "skills": ["alpha"], "min_pass_rate": 0.8, "value_gate_mode": "standard", "baseline_lift_threshold": 0.1, "hard_guard_status": "passed"}},
      "gradingResult": {"pass": false, "score": 0}
    }
  ]
}
JSON

  run node "$ROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  run node - <<'NODE'
const fs = require('fs');
const summary = JSON.parse(fs.readFileSync('site/evals/summary.json', 'utf8'));
if (!summary.aggregates.some((group) => group.providerVariant === 'codex-gpt-5.5' && group.pluginMode === 'full-marketplace')) {
  throw new Error('missing provider variant/plugin mode aggregate');
}
if (!summary.valueGateSummaries.some((gate) => gate.caseId === 'alpha' && gate.providerVariant === 'codex-gpt-5.5' && gate.status === 'pass')) {
  throw new Error(`missing passing value gate: ${JSON.stringify(summary.valueGateSummaries)}`);
}
NODE
  [ "$status" -eq 0 ]

  rm -f "$ROOT/evals/out/results.json" "$ROOT/evals/out/status.json"
  [ ! -f "$backup_dir/results.json" ] || cp "$backup_dir/results.json" "$ROOT/evals/out/results.json"
  [ ! -f "$backup_dir/status.json" ] || cp "$backup_dir/status.json" "$ROOT/evals/out/status.json"
  rm -rf "$backup_dir"
}
