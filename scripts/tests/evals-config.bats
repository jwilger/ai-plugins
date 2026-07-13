#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  GENERATOR="$ROOT/scripts/evals/generate-config.mjs"
  FIXTURE_TMP=""
}

teardown() {
  [ -z "$FIXTURE_TMP" ] || rm -rf "$FIXTURE_TMP"
}

make_codex_only_eval_fixture() {
  FIXTURE_TMP="$(mktemp -d)"
  mkdir -p \
    "$FIXTURE_TMP/scripts/evals" \
    "$FIXTURE_TMP/evals/promptfoo" \
    "$FIXTURE_TMP/.claude-plugin" \
    "$FIXTURE_TMP/.agents/plugins" \
    "$FIXTURE_TMP/plugins/shared/skills/shared-skill" \
    "$FIXTURE_TMP/plugins/codex-only/skills/codex-skill"
  cp "$GENERATOR" "$FIXTURE_TMP/scripts/evals/generate-config.mjs"
  cp "$ROOT/evals/promptfoo/assert-full-marketplace-canary.cjs" "$FIXTURE_TMP/evals/promptfoo/assert-full-marketplace-canary.cjs"
  cat >"$FIXTURE_TMP/evals/matrix.json" <<'JSON'
{
  "providerVariants": [
    {
      "id": "claude-code-sonnet",
      "provider": "anthropic:claude-agent-sdk",
      "modelEnv": "CLAUDE_EVAL_MODEL",
      "defaultModel": "sonnet"
    },
    {
      "id": "codex-gpt-5.6-terra",
      "provider": "openai:codex-sdk",
      "modelEnv": "CODEX_EVAL_MODEL",
      "defaultModel": "gpt-5.6-terra",
      "reasoningEffortEnv": "CODEX_EVAL_REASONING_EFFORT",
      "defaultReasoningEffort": "medium"
    }
  ],
  "pluginModes": [
    {"id": "no-plugins"},
    {"id": "full-marketplace"}
  ]
}
JSON
  cat >"$FIXTURE_TMP/.claude-plugin/marketplace.json" <<'JSON'
{
  "plugins": [
    {
      "name": "shared",
      "source": "./plugins/shared",
      "version": "0.1.0"
    }
  ]
}
JSON
  cat >"$FIXTURE_TMP/.agents/plugins/marketplace.json" <<'JSON'
{
  "plugins": [
    {
      "name": "shared",
      "source": {"source": "local", "path": "./plugins/shared"}
    },
    {
      "name": "codex-only",
      "source": {"source": "local", "path": "./plugins/codex-only"}
    }
  ]
}
JSON
  cat >"$FIXTURE_TMP/plugins/shared/skills/shared-skill/SKILL.md" <<'MD'
---
name: shared-skill
description: Shared skill.
---
MD
  cat >"$FIXTURE_TMP/plugins/codex-only/skills/codex-skill/SKILL.md" <<'MD'
---
name: codex-skill
description: Codex-only skill.
---
MD
}

@test "generated behavior config uses native Promptfoo coding-agent providers" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"openai:codex-sdk"* ]]
  [[ "$output" == *"anthropic:claude-agent-sdk"* ]]
  [[ "$output" == *"Use installed marketplace plugin and skill guidance when it is relevant"* ]]
  [[ "$output" == *"When plugin or skill guidance documents a command, include the exact command name and flags instead of generic setup-path wording."* ]]
  [[ "$output" == *"Do not run shell commands, start evals, mutate files, or inspect repository state."* ]]
  [[ "$output" != *"deep_tracing: true"* ]]
  [[ "$output" == *"deep_tracing: false"* ]]
  [[ "$output" == *"tracing:"*$'\n'"  enabled: false"* ]]
  [[ "$output" == *"Treat each scenario as stateless"* ]]
  [[ "$output" == *"sandbox_mode: read-only"* ]]
  [[ "$output" == *"skip_git_repo_check: true"* ]]
  [[ "$output" == *"working_dir: \"$ROOT/.dependencies/evals/agent-workspace\""* ]]
  [[ "$output" == *"skills: all"* ]]
  [[ "$output" == *"setting_sources: []"* ]]
  [[ "$output" == *"persist_session: false"* ]]
  [[ "$output" == *"disallowed_tools:"*$'\n'"        - Bash"* ]]
  [[ "$output" == *"load-harness-cases.cjs"* ]]
}

@test "generated config uses local Claude Code and Codex auth for providers and graders" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"apiKeyRequired: false"* ]]
  [[ "$output" == *"provider:"*$'\n'"      text:"*$'\n'"        id: openai:codex-sdk"* ]]
  [[ "$output" == *"CODEX_HOME: \"{{ env.CODEX_EVAL_HOME_FULL_MARKETPLACE | default(env.CODEX_EVAL_HOME)"* ]]
  [[ "$output" != *"openai:gpt-5-mini"* ]]
}

@test "generated Codex config defaults execution to Terra and grading to independent Sol" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"model: \"{{ env.CODEX_EVAL_MODEL | default('gpt-5.6-terra') }}\""* ]]
  [[ "$output" == *"model_reasoning_effort: \"{{ env.CODEX_EVAL_REASONING_EFFORT | default('medium') }}\""* ]]
  [[ "$output" == *"model: \"{{ env.CODEX_GRADER_MODEL | default('gpt-5.6-sol') }}\""* ]]
  [[ "$output" == *"model_reasoning_effort: \"{{ env.CODEX_GRADER_REASONING_EFFORT | default('high') }}\""* ]]
}

@test "advisor plugin-eval benchmark defaults to Terra with explicit medium reasoning" {
  benchmark="$ROOT/plugins/advisor/skills/advisor/.plugin-eval/benchmark.json"

  [ "$(jq -r '.runner.model' "$benchmark")" = "gpt-5.6-terra" ]
  jq -e '.runner.extraArgs == ["-c", "model_reasoning_effort=\"medium\""]' "$benchmark"
}

@test "every Codex plugin-eval benchmark defaults to Terra with explicit medium reasoning" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const benchmarkFiles = [];

function visit(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      visit(file);
    } else if (entry.name === 'benchmark.json' && path.basename(path.dirname(file)) === '.plugin-eval') {
      benchmarkFiles.push(file);
    }
  }
}

visit(path.join(root, 'plugins'));
const codexBenchmarks = benchmarkFiles
  .map((file) => ({ file, config: JSON.parse(fs.readFileSync(file, 'utf8')) }))
  .filter(({ config }) => config.runner?.type === 'codex-cli');

if (codexBenchmarks.length === 0) {
  throw new Error('no Codex plugin-eval benchmarks found');
}

const failures = codexBenchmarks.flatMap(({ file, config }) => {
  const errors = [];
  if (config.runner.model !== 'gpt-5.6-terra') {
    errors.push(`model=${JSON.stringify(config.runner.model)}`);
  }
  if (JSON.stringify(config.runner.extraArgs) !== JSON.stringify(['-c', 'model_reasoning_effort="medium"'])) {
    errors.push(`extraArgs=${JSON.stringify(config.runner.extraArgs)}`);
  }
  return errors.length === 0 ? [] : [`${path.relative(root, file)}: ${errors.join(', ')}`];
});

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "operator-facing Promptfoo pins match the package source of truth" {
  promptfoo_version="$(jq -r '.devDependencies.promptfoo' "$ROOT/package.json")"

  for guidance in \
    "$ROOT/plugins/agentic-systems-engineering/skills/scaffold-agentic-evals/SKILL.md" \
    "$ROOT/plugins/agentic-systems-engineering/skills/scaffold-agentic-evals/references/scaffold.md" \
    "$ROOT/plugins/agentic-systems-engineering/bin/promptfoo-mcp"; do
    grep -Fq "promptfoo@$promptfoo_version" "$guidance"
  done
}

@test "generated configs expose every marketplace plugin through the relevant harness surface" {
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
const missing = [];

for (const { name } of claude) {
  if (!result.stdout.includes(path.join(root, 'plugins', name))) {
    missing.push(`claude provider path for ${name}`);
  }
}

for (const { name } of codex) {
  if (!result.stdout.includes(`- name: ${name}`)) {
    missing.push(`metadata entry for ${name}`);
  }
}

if (missing.length > 0) {
  console.error(`missing plugin paths: ${missing.join(', ')}`);
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "generated behavior config uses runtime loader when case filter is set" {
  run env EVAL_CASE_FILTER=tiber node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"evals/out/generated/load-harness-cases.runtime.cjs"* ]]
  [[ "$output" != *"tests: file://$ROOT/evals/promptfoo/load-harness-cases.cjs"* ]]
}

@test "generated Claude provider config excludes Codex-only marketplace plugins" {
  make_codex_only_eval_fixture

  run node - "$FIXTURE_TMP" <<'NODE'
const { spawnSync } = require('child_process');
const path = require('path');

const root = process.argv[2];
const generator = path.join(root, 'scripts/evals/generate-config.mjs');
const result = spawnSync(process.execPath, [generator, '--suite', 'behavior', '--stdout'], {
  cwd: root,
  encoding: 'utf8',
});
if (result.status !== 0) {
  process.stderr.write(result.stderr || result.stdout);
  process.exit(result.status);
}

const firstCodexProvider = result.stdout.indexOf('  - id: openai:codex-sdk');
const claudeSection = result.stdout.slice(0, firstCodexProvider);
const sharedPath = path.join(root, 'plugins/shared');
const codexOnlyPath = path.join(root, 'plugins/codex-only');

if (!claudeSection.includes(sharedPath)) {
  throw new Error(`Claude config did not include shared plugin path: ${sharedPath}`);
}
if (claudeSection.includes(codexOnlyPath)) {
  throw new Error(`Claude config included Codex-only plugin path: ${codexOnlyPath}`);
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

@test "full marketplace canary assertion uses the active provider marketplace" {
  make_codex_only_eval_fixture

  run node - "$FIXTURE_TMP" <<'NODE'
const path = require('path');
process.chdir(process.argv[2]);
const assertCanary = require(path.join(process.argv[2], 'evals/promptfoo/assert-full-marketplace-canary.cjs'));

const claudeResult = assertCanary(
  'Shared: Shared Skill',
  { provider: { id: () => 'anthropic:claude-agent-sdk' } },
);
if (claudeResult.pass !== true) {
  throw new Error(`expected Claude canary to ignore Codex-only plugin: ${JSON.stringify(claudeResult)}`);
}

const codexMissingResult = assertCanary(
  'Shared: Shared Skill',
  { provider: { id: () => 'openai:codex-sdk' } },
);
if (codexMissingResult.pass !== false || !codexMissingResult.reason.includes('codex-only')) {
  throw new Error(`expected Codex canary to require Codex-only plugin: ${JSON.stringify(codexMissingResult)}`);
}

const codexResult = assertCanary(
  'Shared: Shared Skill\nCodex Only: Codex Skill',
  { provider: { id: () => 'openai:codex-sdk' } },
);
if (codexResult.pass !== true) {
  throw new Error(`expected Codex canary to accept Codex-only plugin: ${JSON.stringify(codexResult)}`);
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
  'tiber',
  'worktrees',
  'development-discipline',
].join('\n');

const result = assertCanary(namesOnly, {
  provider: { id: () => 'anthropic:claude-agent-sdk' },
});

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
  'Tiber: Tiber',
  'Worktrees: Setup',
  'Development Discipline: Test Driven Development',
].join('\n');

const result = assertCanary(natural, {
  provider: { id: () => 'anthropic:claude-agent-sdk' },
});

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

@test "codex eval home preparation refreshes stale seeded auth" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  eval_home="$FIXTURE_TMP/eval-home"
  mkdir -p "$auth_home" "$eval_home"
  printf '%s\n' '{"token":"current"}' >"$auth_home/auth.json"
  printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
  printf '%s\n' '{"token":"revoked"}' >"$eval_home/auth.json"

  run env CODEX_EVAL_AUTH_HOME="$auth_home" node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$eval_home" --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  cmp "$auth_home/auth.json" "$eval_home/auth.json"
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
