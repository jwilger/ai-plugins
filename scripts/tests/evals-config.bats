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
    "$FIXTURE_TMP/evals/fixtures/behavior" \
    "$FIXTURE_TMP/.claude-plugin" \
    "$FIXTURE_TMP/.agents/plugins" \
    "$FIXTURE_TMP/plugins/development-system/skills/shared-skill" \
    "$FIXTURE_TMP/plugins/codex-only/skills/codex-skill"
  cp "$GENERATOR" "$FIXTURE_TMP/scripts/evals/generate-config.mjs"
  cp "$ROOT/evals/promptfoo/assert-development-system-canary.cjs" "$FIXTURE_TMP/evals/promptfoo/assert-development-system-canary.cjs"
  cp "$ROOT/evals/promptfoo/fixtures.cjs" "$FIXTURE_TMP/evals/promptfoo/fixtures.cjs"
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
    {"id": "development-system"}
  ]
}
JSON
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {
    "case_id": "shared-case",
    "plugins": ["development-system"]
  }
]
JSON
  cat >"$FIXTURE_TMP/.claude-plugin/marketplace.json" <<'JSON'
{
  "plugins": [
    {
      "name": "development-system",
      "source": "./plugins/development-system",
      "version": "0.1.0"
    }
  ]
}
JSON
  cat >"$FIXTURE_TMP/.agents/plugins/marketplace.json" <<'JSON'
{
  "plugins": [
    {
      "name": "development-system",
      "source": {"source": "local", "path": "./plugins/development-system"},
      "version": "0.1.0"
    },
    {
      "name": "codex-only",
      "source": {"source": "local", "path": "./plugins/codex-only"}
    }
  ]
}
JSON
  cat >"$FIXTURE_TMP/plugins/development-system/skills/shared-skill/SKILL.md" <<'MD'
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
  [[ "$output" == *"Use only context made available inside this evaluation condition."* ]]
  [[ "$output" == *"Do not rely on prior conversations, user memory, session memory, earlier runs, or host-machine state."* ]]
  [[ "$output" != *"installed marketplace plugin"* ]]
  [[ "$output" != *"exact command name"* ]]
  [[ "$output" != *"deep_tracing: true"* ]]
  [[ "$output" == *"deep_tracing: false"* ]]
  [[ "$output" == *"tracing:"*$'\n'"  enabled: false"* ]]
  [[ "$output" == *"stateless advisory question"* ]]
  [[ "$output" == *"sandbox_mode: read-only"* ]]
  [[ "$output" == *"skip_git_repo_check: true"* ]]
  [[ "$output" == *"codex_path_override: \"$ROOT/scripts/evals/behavior-provider-boundary.sh\""* ]]
  [[ "$output" == *"EVAL_PROVIDER_CODEX_RUNTIME:"* ]]
  [[ "$output" == *"working_dir: \"/tmp/ai-plugins-provider-eval-"* ]]
  [[ "$output" == *"skills: all"* ]]
  [[ "$output" == *"setting_sources: []"* ]]
  [[ "$output" == *"persist_session: false"* ]]
  [[ "$output" == *"disallowed_tools:"*$'\n'"        - Bash"* ]]
  [[ "$output" == *"        - WebSearch"* ]]
  [[ "$output" == *"        - WebFetch"* ]]
  [[ "$output" == *"load-harness-cases.cjs"* ]]
}

@test "generated Claude providers append dispatch tools without stripping worker writability" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^      append_allowed_tools:$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - Agent$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - Skill$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^      disallowed_tools:$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - Bash$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - WebSearch$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - WebFetch$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - Write$')" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - Edit$')" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        - MultiEdit$')" -eq 0 ]
  [[ "$output" != *"custom_allowed_tools:"* ]]
}

@test "behavior eval matrix keeps isolated Claude Code and Codex conditions" {
  run jq -e '
    [.pluginModes[].id] == ["no-plugins", "development-system"] and
    [.providerVariants[].id] == ["claude-code-sonnet", "codex-gpt-5.6-terra"] and
    all(.providerVariants[]; .pluginModes == ["no-plugins", "development-system"]) and
    all(.providerVariants[]; .provider == "anthropic:claude-agent-sdk" or .provider == "openai:codex-sdk")
  ' "$ROOT/evals/matrix.json"

  [ "$status" -eq 0 ]
}

@test "native behavior providers use neutral prompts and fail-closed filesystem isolation with leak rejection" {
  FIXTURE_TMP="$(mktemp -d)"
  generated_config="$FIXTURE_TMP/config.yaml"
  generated_metadata="$FIXTURE_TMP/metadata.json"

  run node "$GENERATOR" \
    --suite behavior \
    --output "$generated_config" \
    --metadata-output "$generated_metadata"

  [ "$status" -eq 0 ]

  run node - "$ROOT" "$generated_config" "$generated_metadata" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');

const [root, configPath, metadataPath] = process.argv.slice(2);
const config = fs.readFileSync(configPath, 'utf8');
const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
const prompt = config.slice(config.indexOf('prompts:'), config.indexOf('\nproviders:'));
const leaksExpectedAnswer = [
  /installed marketplace plugin/i,
  /skill guidance/i,
  /exact command name/i,
  /plugin-specific/i,
].some((pattern) => pattern.test(prompt));
if (leaksExpectedAnswer) {
  throw new Error('baseline prompt reveals plugin-specific guidance or expected-answer shape');
}

const providerSections = config
  .split(/^  - id: /m)
  .slice(1)
  .map((section) => `  - id: ${section}`);
if (providerSections.length !== 4) {
  throw new Error(`expected four native provider conditions, found ${providerSections.length}`);
}
for (const section of providerSections) {
  const label = section.match(/^\s*label:\s*(\S+)\s*$/m)?.[1];
  if (!label) {
    throw new Error('native provider condition lacks a label');
  }
  const workingDir = section.match(/^\s*working_dir:\s*"([^"]+)"\s*$/m)?.[1];
  if (!workingDir || !path.isAbsolute(workingDir)) {
    throw new Error(`${label} working_dir is not absolute`);
  }
  const relative = path.relative(root, workingDir);
  if (relative === '' || (!relative.startsWith('..' + path.sep) && relative !== '..')) {
    throw new Error(`${label} working_dir remains beneath repository instructions: ${workingDir}`);
  }
  const executable = section.match(
    /^\s*(?:codex_path_override|path_to_claude_code_executable|executable(?:_path)?):\s*"([^"]+)"\s*$/m,
  )?.[1];
  if (!executable || !path.isAbsolute(executable)) {
    throw new Error(`${label} lacks an absolute executable boundary override`);
  }
}

if (metadata.providerCompositions.length !== 4) {
  throw new Error(`expected four provider compositions, found ${metadata.providerCompositions.length}`);
}
for (const composition of metadata.providerCompositions) {
  const isolation = composition.filesystemIsolation;
  const expectedPluginMounts =
    composition.pluginMode === 'development-system' ? ['development-system'] : [];
  if (
    isolation?.boundary !== 'mount-namespace' ||
    isolation?.failClosed !== true ||
    isolation?.workspace !== 'outside-repository' ||
    isolation?.repository !== 'hidden' ||
    isolation?.globalGuidance !== 'hidden' ||
    isolation?.globalHomes !== 'hidden' ||
    isolation?.globalCaches !== 'hidden' ||
    isolation?.globalCatalogs !== 'hidden' ||
    isolation?.conditionHome !== 'owned' ||
    !Array.isArray(isolation?.pluginMounts) ||
    JSON.stringify(isolation.pluginMounts) !== JSON.stringify(expectedPluginMounts) ||
    isolation?.leakagePolicy !== 'reject'
  ) {
    throw new Error(`incomplete ${composition.label} filesystem isolation: ${JSON.stringify(isolation)}`);
  }
}
NODE
  [ "$status" -eq 0 ]
}

@test "behavior thresholds reject output or traces containing isolation leak markers" {
  FIXTURE_TMP="$(mktemp -d)"
  results="$FIXTURE_TMP/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": {"label": "codex-gpt-5.6-terra-no-plugins"},
        "testCase": {
          "vars": {
            "case_id": "context-leak-canary",
            "plugin_mode": "no-plugins",
            "min_pass_rate": 1,
            "value_gate_mode": "measurement",
            "context_leak_markers": ["PRIVATE_PLUGIN_GUIDANCE_CANARY"]
          }
        },
        "gradingResult": {"pass": true, "score": 1},
        "response": {"output": "Use PRIVATE_PLUGIN_GUIDANCE_CANARY exactly."},
        "metadata": {
          "trace": "read /host/repository/plugins/development-system/skills/setup/SKILL.md"
        }
      }
    ]
  }
}

JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  [ "$status" -ne 0 ]
  [[ "$output" == *"context leak"* || "$output" == *"context-leak"* ]]
  [[ "$output" == *"context-leak-canary"* ]]
}

@test "behavior loader feeds canonical host-only markers into threshold rejection" {
  FIXTURE_TMP="$(mktemp -d)"
  results="$FIXTURE_TMP/results.json"
  marker_file="$FIXTURE_TMP/context-leak-markers.json"

  env EVAL_RUNTIME_OPTIONS_FILE="$FIXTURE_TMP/no-runtime-options.json" \
    EVAL_PROVIDER_WORKSPACE="$FIXTURE_TMP/host-workspace" \
    node - "$ROOT" "$results" "$marker_file" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const [root, resultsPath, markerFile] = process.argv.slice(2);
const generateTests = require(path.join(root, 'evals/promptfoo/load-harness-cases.cjs'));
const { contextLeakMarkers } = require(path.join(root, 'evals/promptfoo/fixtures.cjs'));
const generated = generateTests()[0];
const markers = contextLeakMarkers();
const marker = markers.find(
  (candidate) => candidate === path.resolve(process.env.EVAL_PROVIDER_WORKSPACE),
);
if (!marker) throw new Error('private sidecar omitted the generated host workspace marker');
if (JSON.stringify(generated.vars).includes(marker)) {
  throw new Error('loader embedded a host workspace marker in shareable vars');
}
fs.writeFileSync(
  markerFile,
  JSON.stringify({ version: 1, markers, secrets: [marker] }),
  { mode: 0o600 },
);

fs.writeFileSync(
  resultsPath,
  JSON.stringify({
    results: {
      results: [
        {
          provider: { label: 'codex-gpt-5.6-terra-no-plugins' },
          testCase: { vars: generated.vars },
          gradingResult: { pass: true, score: 1 },
          metadata: { trace: `provider read ${marker}/private-guidance` },
        },
      ],
    },
  }),
);
NODE

  run env EVAL_CONTEXT_LEAK_MARKERS_FILE="$marker_file" \
    EVAL_REQUIRE_CONTEXT_LEAK_SIDECAR=1 \
    node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  [ "$status" -ne 0 ]
  [[ "$output" == *"Context leak failures"* ]]
}

@test "behavior loader keeps dynamic host paths out of shareable result variables" {
  FIXTURE_TMP="$(mktemp -d)"
  host_workspace="$FIXTURE_TMP/private-host-workspace"

  run env EVAL_RUNTIME_OPTIONS_FILE="$FIXTURE_TMP/no-runtime-options.json" \
    EVAL_PROVIDER_WORKSPACE="$host_workspace" \
    node - "$ROOT" "$host_workspace" <<'NODE'
const path = require('node:path');
const [root, hostWorkspace] = process.argv.slice(2);
const generateTests = require(path.join(root, 'evals/promptfoo/load-harness-cases.cjs'));
const serialized = JSON.stringify(generateTests().map((testCase) => testCase.vars));
for (const privatePath of [path.resolve(root), path.resolve(process.env.HOME), path.resolve(hostWorkspace)]) {
  if (serialized.includes(privatePath)) {
    throw new Error(`shareable fixture variables contain host path: ${privatePath}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "behavior thresholds fail closed when fixture leak markers are missing" {
  FIXTURE_TMP="$(mktemp -d)"
  results="$FIXTURE_TMP/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": {"label": "codex-gpt-5.6-terra-no-plugins"},
        "testCase": {
          "vars": {
            "case_id": "missing-context-leak-markers",
            "fixture_file": "evals/fixtures/behavior/example/cases.json",
            "plugin_mode": "no-plugins",
            "min_pass_rate": 1,
            "value_gate_mode": "measurement"
          }
        },
        "gradingResult": {"pass": true, "score": 1}
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  [ "$status" -ne 0 ]
  [[ "$output" == *"Context leak configuration failures"* ]]
  [[ "$output" == *"missing-context-leak-markers"* ]]
}

@test "generated behavior providers compare no plugins with installed development-system" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^    label: ')" -eq 4 ]
  [[ "$output" == *"label: claude-code-sonnet-no-plugins"* ]]
  [[ "$output" == *"label: claude-code-sonnet-development-system"* ]]
  [[ "$output" == *"label: codex-gpt-5.6-terra-no-plugins"* ]]
  [[ "$output" == *"label: codex-gpt-5.6-terra-development-system"* ]]
  [[ "$output" != *"targeted-plugins"* ]]
  [[ "$output" != *"pi-provider.mjs"* ]]
  [[ "$output" != *"full-marketplace"* ]]
  [[ "$output" == *'path: "/runtime/plugin"'* ]]
  [[ "$output" == *"EVAL_PROVIDER_PLUGIN_SNAPSHOT:"* ]]
  [[ "$output" != *"path: \"$ROOT/plugins/development-system\""* ]]
}

@test "generated behavior config keeps all eval runtime state outside dependencies" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"$ROOT/.evals/codex-home-development-system"* && "$output" != *"$ROOT/.dependencies/evals/"* ]]
}

@test "generated config uses local Claude Code and Codex auth for providers and graders" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"apiKeyRequired: false"* ]]
  [[ "$output" == *'CLAUDE_CONFIG_DIR: "/runtime/home/config"'* ]]
  [[ "$output" == *"provider:"*$'\n'"      text:"*$'\n'"        id: openai:codex-sdk"* ]]
  [[ "$output" == *'CODEX_HOME: "/runtime/home"'* ]]
  [[ "$output" != *"openai:gpt-5-mini"* ]]
}

@test "generated semantic grader uses a dedicated sanitized no-plugin condition" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  grader="${output#*defaultTest:}"
  [[ "$grader" == *'EVAL_PLUGIN_MODE: "no-plugins"'* ]]
  [[ "$grader" == *'EVAL_PROVIDER_HOME: "{{ env.CODEX_EVAL_HOME_GRADER'* ]]
  [[ "$grader" == *"$ROOT/.evals/codex-home-grader"* ]]
  [[ "$grader" == *"workspaces/codex-grader"* ]]
  [[ "$grader" != *"EVAL_PROVIDER_PLUGIN_SNAPSHOT"* ]]
  [[ "$grader" != *"codex-home-development-system"* ]]
  [[ "$grader" != *"sanitized-marketplace"* ]]
}

@test "generated Codex config defaults execution to Terra and grading to independent Sol" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"model: \"{{ env.CODEX_EVAL_MODEL | default('gpt-5.6-terra') }}\""* ]]
  [[ "$output" == *"model_reasoning_effort: \"{{ env.CODEX_EVAL_REASONING_EFFORT | default('medium') }}\""* ]]
  [[ "$output" == *"model: \"{{ env.CODEX_GRADER_MODEL | default('gpt-5.6-sol') }}\""* ]]
  [[ "$output" == *"model_reasoning_effort: \"{{ env.CODEX_GRADER_REASONING_EFFORT | default('high') }}\""* ]]
}

@test "generated Codex providers enable coding collaboration with eight agent threads" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^      collaboration_mode: coding$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^      cli_config:$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^        features:$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^          expose_spawn_agent_model_overrides: true$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^          multi_agent_v2:$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^            enabled: true$')" -eq 2 ]
  [ "$(printf '%s\n' "$output" | grep -c '^            max_concurrent_threads_per_session: 8$')" -eq 2 ]
}

@test "generated behavior config uses runtime loader when case filter is set" {
  run env EVAL_CASE_FILTER=beads node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"evals/out/generated/load-harness-cases.runtime.cjs"* ]]
  [[ "$output" != *"tests: file://$ROOT/evals/promptfoo/load-harness-cases.cjs"* ]]
}

@test "eval runner exports capability options for static and generated loaders" {
  run grep -F 'export EVAL_RUNTIME_OPTIONS_FILE="$runtime_options_file"' \
    "$ROOT/scripts/evals/run.sh"

  [ "$status" -eq 0 ]
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
const sharedPath = path.join(
  root,
  '.evals/claude-home-development-system/plugin-cache/cache/ai-plugins/development-system/0.1.0',
);
const codexOnlyPath = path.join(root, 'plugins/codex-only');

if (!claudeSection.includes(sharedPath)) {
  throw new Error(`Claude config did not include installed development-system path: ${sharedPath}`);
}
if (claudeSection.includes(codexOnlyPath)) {
  throw new Error(`Claude config included Codex-only plugin path: ${codexOnlyPath}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "generated config fails when development-system is unavailable to a harness" {
  make_codex_only_eval_fixture
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {
    "case_id": "codex-only-case",
    "plugins": ["codex-only"]
  }
]
JSON

  jq 'del(.plugins[] | select(.name == "development-system"))' \
    "$FIXTURE_TMP/.claude-plugin/marketplace.json" \
    >"$FIXTURE_TMP/.claude-plugin/marketplace.json.tmp"
  mv "$FIXTURE_TMP/.claude-plugin/marketplace.json.tmp" \
    "$FIXTURE_TMP/.claude-plugin/marketplace.json"

  run node "$FIXTURE_TMP/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -ne 0 ]
  [[ "$output" == *"Claude Code marketplace must contain exactly one development-system plugin"* ]]
  [[ "$output" != *$'\nproviders:\n'* ]]
}

@test "generated Claude plugin path exposes only the boundary-mounted snapshot" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.claude-plugin/plugin.json")"
  [[ "$output" == *'path: "/runtime/plugin"'* ]]
  [[ "$output" == *"EVAL_PROVIDER_PLUGIN_SNAPSHOT:"* ]]
  [[ "$output" == *"$ROOT/.evals/claude-home-development-system/plugin-cache/cache/ai-plugins/development-system/$version"* ]]
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
if (!tests.some((testCase) => testCase.description === 'development-system-canary')) {
  throw new Error('missing development-system-canary test');
}
if (!tests.some((testCase) => testCase.vars?.scenario_prompt?.includes('Do not inspect repository files'))) {
  throw new Error('canary should answer from loaded harness context, not repository file reads');
}
if (tests.some((testCase) => (testCase.assert || []).some((assertion) => assertion.type === 'skill-used'))) {
  throw new Error('canary must not depend on skill-used because Codex plugin-cache skills are not reported there');
}
if (!tests.some((testCase) => (testCase.assert || []).some((assertion) => assertion.type === 'javascript' && assertion.value.includes('assert-development-system-canary.cjs')))) {
  throw new Error('missing development-system canary assertion');
}
NODE

  [ "$status" -eq 0 ]
}

@test "development-system canary assertion uses the active provider marketplace" {
  make_codex_only_eval_fixture

  run node - "$FIXTURE_TMP" <<'NODE'
const path = require('path');
process.chdir(process.argv[2]);
const assertCanary = require(path.join(process.argv[2], 'evals/promptfoo/assert-development-system-canary.cjs'));

const claudeResult = assertCanary(
  'Development System: Shared Skill',
  { provider: { id: () => 'anthropic:claude-agent-sdk' } },
);
if (claudeResult.pass !== true) {
  throw new Error(`expected Claude canary to ignore Codex-only plugin: ${JSON.stringify(claudeResult)}`);
}

const codexMissingResult = assertCanary(
  'Development System',
  { provider: { id: () => 'openai:codex-sdk' } },
);
if (codexMissingResult.pass !== false || !codexMissingResult.reason.includes('representative skill')) {
  throw new Error(`expected Codex canary to require a development-system skill: ${JSON.stringify(codexMissingResult)}`);
}

const codexResult = assertCanary(
  'Development System: Shared Skill',
  { provider: { id: () => 'openai:codex-sdk' } },
);
if (codexResult.pass !== true) {
  throw new Error(`expected Codex canary to accept development-system: ${JSON.stringify(codexResult)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "development-system canary requires representative skills, not only plugin names" {
  run node - <<'NODE'
const assertCanary = require('./evals/promptfoo/assert-development-system-canary.cjs');
const namesOnly = [
  'development-system',
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

@test "development-system canary accepts natural title-cased skill names" {
  run node - <<'NODE'
const assertCanary = require('./evals/promptfoo/assert-development-system-canary.cjs');
const natural = [
  'Agentic Systems Engineering: Evaluate Stochastic Systems',
  'Babysit PR: Babysit PR',
  'Engineering Standards: Engineering Standards',
  'Eval Case Reporter: Submit Eval Case',
  'Beads: Beads',
  'Worktrees: Setup',
  'Development Discipline: Test Driven Development',
  'Development System: Setup',
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

@test "codex eval home preparation installs development-system into cache" {
  tmp_home="$(mktemp -d)"

  run node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$tmp_home"

  [ "$status" -eq 0 ]
  grep -q '\[marketplaces.ai-plugins\]' "$tmp_home/config.toml"

  grep -q '\[plugins\."development-system@ai-plugins"\]' "$tmp_home/config.toml"
  [ -d "$tmp_home/plugins/cache/ai-plugins/development-system" ]

  rm -rf "$tmp_home"
}

@test "codex live eval home preparation installs through the Codex CLI" {
  FIXTURE_TMP="$(mktemp -d)"
  eval_home="$FIXTURE_TMP/eval-home"
  fake_codex="$FIXTURE_TMP/codex"
  invocation_log="$FIXTURE_TMP/invocations"
  cat >"$fake_codex" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$CODEX_CLI_INVOCATION_LOG"
case "$*" in
  "plugin marketplace add "*" --json")
    mkdir -p "$CODEX_HOME"
    cat >"$CODEX_HOME/config.toml" <<EOF
[marketplaces.ai-plugins]
source_type = "local"
source = "$CODEX_CLI_PLUGIN_ROOT"
EOF
    printf '{"marketplaceName":"ai-plugins"}\n'
    ;;
  "plugin add development-system@ai-plugins --json")
    version="$(jq -r '.version' "$CODEX_CLI_PLUGIN_ROOT/plugins/development-system/.codex-plugin/plugin.json")"
    installed="$CODEX_HOME/plugins/cache/ai-plugins/development-system/$version"
    mkdir -p "$installed"
    cp -R "$CODEX_CLI_PLUGIN_ROOT/plugins/development-system/." "$installed/"
    printf 'installed-by-codex-cli\n' >"$installed/cli-install-artifact"
    cat >>"$CODEX_HOME/config.toml" <<'EOF'

[plugins."development-system@ai-plugins"]
enabled = true
EOF
    printf '{"pluginId":"development-system@ai-plugins","installedPath":"%s"}\n' "$installed"
    ;;
  "plugin list --json")
    printf '{"installed":[{"pluginId":"development-system@ai-plugins","installed":true,"enabled":true}],"available":[]}\n'
    ;;
  *)
    printf 'unexpected Codex CLI arguments: %s\n' "$*" >&2
    exit 64
    ;;
esac
SH
  chmod +x "$fake_codex"

  run env \
    OPENAI_API_KEY=fixture \
    CODEX_EVAL_CODEX_BIN="$fake_codex" \
    CODEX_CLI_INVOCATION_LOG="$invocation_log" \
    CODEX_CLI_PLUGIN_ROOT="$ROOT" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode development-system \
    --install-via-cli

  [ "$status" -eq 0 ]
  [ "$(sed -n '1p' "$invocation_log")" = "plugin marketplace add $ROOT --json" ]
  [ "$(sed -n '2p' "$invocation_log")" = "plugin add development-system@ai-plugins --json" ]
  [ "$(sed -n '3p' "$invocation_log")" = "plugin list --json" ]
  grep -q '\[plugins\."development-system@ai-plugins"\]' "$eval_home/config.toml"
  [ -d "$eval_home/plugins/cache/ai-plugins/development-system" ]
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.codex-plugin/plugin.json")"
  grep -q '^installed-by-codex-cli$' \
    "$eval_home/plugins/cache/ai-plugins/development-system/$version/cli-install-artifact"
  grep -q '^installed-by-codex-cli$' \
    "$eval_home/sanitized-marketplace/plugins/development-system/cli-install-artifact"
  [ ! -e "$ROOT/plugins/development-system/cli-install-artifact" ]
}

@test "codex eval home preparation refreshes stale seeded auth" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  eval_home="$FIXTURE_TMP/eval-home"
  mkdir -p "$auth_home" "$eval_home"
  printf '%s\n' '{"token":"current"}' >"$auth_home/auth.json"
  printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
  printf '%s\n' '{"token":"revoked"}' >"$eval_home/auth.json"

  run env -u OPENAI_API_KEY -u CODEX_API_KEY \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  cmp "$auth_home/auth.json" "$eval_home/auth.json"
}

@test "codex eval home preparation records seeded auth scalars for artifact scanning" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  eval_home="$FIXTURE_TMP/eval-home"
  secret_manifest="$FIXTURE_TMP/artifact-secrets.json"
  mkdir -p "$auth_home" "$eval_home"
  printf '%s\n' '{
    "auth_mode":"chatgpt",
    "tokens":{
      "access_token":"fixture-codex-access-value",
      "id_token":"fixture-codex-id-value",
      "refresh_token":"fixture-codex-refresh-value"
    },
    "agent_identity":{
      "agent_private_key":"fixture-agent-private-key-value"
    },
    "future_auth":{
      "session_secret":"fixture-future-session-secret",
      "display_name":"not-sensitive"
    }
  }' >"$auth_home/auth.json"
  printf '%s\n' '{
    "legacyCredential":"fixture-legacy-credential-value",
    "label":"not-sensitive-either"
  }' >"$auth_home/.credentials.json"

  run env -u OPENAI_API_KEY -u CODEX_API_KEY \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins \
    --artifact-secret-output "$secret_manifest"

  [ "$status" -eq 0 ]
  [ "$(stat -c '%a' "$secret_manifest")" = 600 ]
  jq -e '
    .version == 1 and
    (.secrets | sort) == ([
      "fixture-codex-access-value",
      "fixture-codex-id-value",
      "fixture-codex-refresh-value",
      "fixture-agent-private-key-value",
      "fixture-future-session-secret",
      "fixture-legacy-credential-value"
    ] | sort)
  ' "$secret_manifest" >/dev/null
  ! rg -Fq 'not-sensitive' "$secret_manifest"
}

@test "codex auth scan metadata unions secrets across repeated home preparation" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  first_home="$FIXTURE_TMP/first-home"
  second_home="$FIXTURE_TMP/second-home"
  secret_manifest="$FIXTURE_TMP/artifact-secrets.json"
  mkdir -p "$auth_home" "$first_home" "$second_home"
  printf '%s\n' '{"tokens":{"access_token":"first-preparation-secret"}}' \
    >"$auth_home/auth.json"

  run env -u OPENAI_API_KEY -u CODEX_API_KEY \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$first_home" --plugin-mode no-plugins \
    --artifact-secret-output "$secret_manifest"

  [ "$status" -eq 0 ]
  printf '%s\n' '{"tokens":{"access_token":"second-preparation-secret"}}' \
    >"$auth_home/auth.json"

  run env -u OPENAI_API_KEY -u CODEX_API_KEY \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$second_home" --plugin-mode no-plugins \
    --artifact-secret-output "$secret_manifest"

  [ "$status" -eq 0 ]
  jq -e '
    (.secrets | sort) == ([
      "first-preparation-secret",
      "second-preparation-secret"
    ] | sort)
  ' "$secret_manifest" >/dev/null
}

@test "codex eval home preparation fails closed when seeded auth cannot produce scan metadata" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  eval_home="$FIXTURE_TMP/eval-home"
  secret_manifest="$FIXTURE_TMP/artifact-secrets.json"
  mkdir -p "$auth_home" "$eval_home"
  printf '%s\n' '{not-json' >"$auth_home/auth.json"

  run env -u OPENAI_API_KEY -u CODEX_API_KEY \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins \
    --artifact-secret-output "$secret_manifest"

  [ "$status" -eq 2 ]
  [ ! -e "$secret_manifest" ]
  [[ "$output" != *"fixture"* ]]
}

@test "codex eval home preparation can omit all copied auth material" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  eval_home="$FIXTURE_TMP/eval-home"
  mkdir -p "$auth_home" "$eval_home"
  printf '%s\n' '{"token":"oauth-secret"}' >"$auth_home/auth.json"
  printf '%s\n' '{"token":"credential-secret"}' \
    >"$auth_home/.credentials.json"
  printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
  printf '%s\n' '{"token":"stale-oauth-secret"}' >"$eval_home/auth.json"
  printf '%s\n' '{"token":"stale-credential-secret"}' \
    >"$eval_home/.credentials.json"

  run env -u OPENAI_API_KEY -u CODEX_API_KEY \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins \
    --no-seed-auth

  [ "$status" -eq 0 ]
  [ -f "$eval_home/config.toml" ]
  [ ! -e "$eval_home/auth.json" ]
  [ ! -e "$eval_home/.credentials.json" ]
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
