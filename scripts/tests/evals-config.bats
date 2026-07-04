#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  GENERATOR="$ROOT/scripts/evals/generate-config.mjs"
}

@test "generated behavior config uses native Promptfoo coding-agent providers" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"openai:codex-sdk"* ]]
  [[ "$output" == *"anthropic:claude-agent-sdk"* ]]
  [[ "$output" == *"deep_tracing: true"* ]]
  [[ "$output" == *"sandbox_mode: read-only"* ]]
  [[ "$output" == *"skills: all"* ]]
  [[ "$output" == *"load-harness-cases.cjs"* ]]
}

@test "generated config uses local Claude Code and Codex auth for providers and graders" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"apiKeyRequired: false"* ]]
  [[ "$output" == *"provider:"*$'\n'"      id: openai:codex-sdk"* ]]
  [[ "$output" == *"CODEX_HOME: \"{{ env.CODEX_EVAL_HOME"* ]]
  [[ "$output" != *"openai:gpt-5-mini"* ]]
}

@test "generated configs load every marketplace plugin path" {
  run node - "$ROOT" "$GENERATOR" <<'NODE'
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = process.argv[2];
const generator = process.argv[3];
const result = spawnSync(process.execPath, [generator, '--suite', 'behavior', '--stdout'], {
  cwd: root,
  encoding: 'utf8',
});
if (result.status !== 0) {
  process.stderr.write(result.stderr || result.stdout);
  process.exit(result.status);
}

const claude = JSON.parse(fs.readFileSync(path.join(root, '.claude-plugin/marketplace.json'), 'utf8')).plugins;
const codex = JSON.parse(fs.readFileSync(path.join(root, '.agents/plugins/marketplace.json'), 'utf8')).plugins;
const names = new Set([...claude, ...codex].map((plugin) => plugin.name));
const missing = [];

for (const name of names) {
  if (!result.stdout.includes(path.join(root, 'plugins', name))) {
    missing.push(name);
  }
}

if (missing.length > 0) {
  console.error(`missing plugin paths: ${missing.join(', ')}`);
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "generated claude plugin paths are absolute so generated configs can move" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"path: \"$ROOT/plugins/agentic-systems-engineering\""* ]]
  [[ "$output" != *"path: \"./plugins/"* ]]
}

@test "generated canary config is separate from natural behavior scenarios" {
  run node "$GENERATOR" --suite canary --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"load-canary-cases.cjs"* ]]
  [[ "$output" != *"We ran our new LLM workflow once in a demo"* ]]

  run node - <<'NODE'
const generateTests = require('./evals/promptfoo/load-canary-cases.cjs');
const tests = generateTests();
if (!tests.some((testCase) => testCase.description === 'full-marketplace-canary')) {
  throw new Error('missing full-marketplace-canary test');
}
if (!tests.some((testCase) => testCase.vars?.scenario_prompt?.includes('Do not inspect repository files'))) {
  throw new Error('canary should answer from loaded harness context, not repository file reads');
}
if (tests.some((testCase) => (testCase.assert || []).some((assertion) => assertion.type === 'skill-used'))) {
  throw new Error('canary must not depend on skill-used because Codex plugin-cache skills are not reported there');
}
if (!tests.some((testCase) => (testCase.assert || []).some((assertion) => assertion.type === 'javascript' && assertion.value.includes('assert-full-marketplace-canary.cjs')))) {
  throw new Error('missing full-marketplace canary assertion');
}
NODE

  [ "$status" -eq 0 ]
}

@test "full marketplace canary requires representative skills, not only plugin names" {
  run node - <<'NODE'
const assertCanary = require('./evals/promptfoo/assert-full-marketplace-canary.cjs');
const namesOnly = [
  'agentic-systems-engineering',
  'babysit-pr',
  'engineering-standards',
  'eval-case-reporter',
  'worktrees',
].join('\n');

const result = assertCanary(namesOnly);

if (result.pass !== false || !result.reason.includes('representative skill')) {
  throw new Error(`expected skill-level canary failure, got: ${JSON.stringify(result)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "full marketplace canary accepts natural title-cased skill names" {
  run node - <<'NODE'
const assertCanary = require('./evals/promptfoo/assert-full-marketplace-canary.cjs');
const natural = [
  'Agentic Systems Engineering: Evaluate Stochastic Systems',
  'Babysit PR: Babysit PR',
  'Engineering Standards: Engineering Standards',
  'Eval Case Reporter: Submit Eval Case',
  'Worktrees: Setup',
].join('\n');

const result = assertCanary(natural);

if (result.pass !== true) {
  throw new Error(`expected title-cased skills to pass, got: ${JSON.stringify(result)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "codex eval home preparation installs all marketplace plugins into cache" {
  tmp_home="$(mktemp -d)"

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$tmp_home"

  [ "$status" -eq 0 ]
  grep -q '\[marketplaces.ai-plugins\]' "$tmp_home/config.toml"

  while IFS= read -r plugin; do
    grep -q "\\[plugins\\.\"${plugin}@ai-plugins\"\\]" "$tmp_home/config.toml"
    [ -d "$tmp_home/plugins/cache/ai-plugins/$plugin" ]
  done < <(jq -r '.plugins[].name' "$ROOT/.agents/plugins/marketplace.json")

  rm -rf "$tmp_home"
}

@test "codex eval home preparation refuses the real codex home by default" {
  tmp_home="$(mktemp -d)"

  run env HOME="$tmp_home" node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$tmp_home/.codex"

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to prepare real Codex home"* ]]

  rm -rf "$tmp_home"
}

@test "codex eval home preparation refuses symlinks to the real codex home" {
  tmp_home="$(mktemp -d)"
  mkdir -p "$tmp_home/.codex"
  ln -s "$tmp_home/.codex" "$tmp_home/eval-home-link"

  run env HOME="$tmp_home" node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$tmp_home/eval-home-link"

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to prepare real Codex home"* ]]

  rm -rf "$tmp_home"
}

@test "codex eval home preparation refuses to overwrite the auth source home" {
  tmp_home="$(mktemp -d)"
  mkdir -p "$tmp_home/custom-codex"

  run env HOME="$tmp_home" CODEX_HOME="$tmp_home/custom-codex" node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$tmp_home/custom-codex"

  [ "$status" -ne 0 ]
  [[ "$output" == *"refusing to prepare auth source Codex home"* ]]

  rm -rf "$tmp_home"
}
