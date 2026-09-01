#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  GENERATOR="$ROOT/scripts/evals/generate-config.mjs"
  FIXTURE_TMP=""
}

teardown() {
  [ -z "$FIXTURE_TMP" ] || rm -rf "$FIXTURE_TMP"
}

make_codex_eval_fixture() {
  FIXTURE_TMP="$(mktemp -d)"
  mkdir -p \
    "$FIXTURE_TMP/scripts/evals" \
    "$FIXTURE_TMP/evals/promptfoo" \
    "$FIXTURE_TMP/evals/fixtures/behavior" \
    "$FIXTURE_TMP/.agents/plugins" \
    "$FIXTURE_TMP/plugins/shared/skills/shared-skill" \
    "$FIXTURE_TMP/plugins/codex-only/skills/codex-skill"
  cp "$GENERATOR" "$FIXTURE_TMP/scripts/evals/generate-config.mjs"
  cp "$ROOT/evals/promptfoo/assert-full-marketplace-canary.cjs" "$FIXTURE_TMP/evals/promptfoo/assert-full-marketplace-canary.cjs"
  cp "$ROOT/evals/promptfoo/fixtures.cjs" "$FIXTURE_TMP/evals/promptfoo/fixtures.cjs"
  cat >"$FIXTURE_TMP/evals/matrix.json" <<'JSON'
{
  "providerVariants": [
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
    {"id": "targeted-plugins"},
    {"id": "full-marketplace"}
  ]
}
JSON
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {
    "case_id": "shared-case",
    "plugins": ["shared"]
  }
]
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

@test "generated behavior config uses native providers and a neutral advisory prefix" {
  local expected_eval_workspace
  expected_eval_workspace="$(node -p "require('path').join(require('os').tmpdir(), 'ai-plugins-provider-eval-workspace')")"
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"openai:codex-sdk"* ]]
  [[ "$output" == *"Answer the scenario directly as a stateless advisory question"* ]]
  [[ "$output" == *"If you recommend a command, give its exact name and flags"* ]]
  [[ "$output" == *"Apply any available instructions relevant to the scenario."* ]]
  [[ "$output" == *"Do not mention hidden eval scaffolding"* ]]
  [[ "$output" == *"name the applicable skills or specialist contracts"* ]]
  [[ "$output" == *"Keep advisory answers at the requested level"* ]]
  [[ "$output" != *"Use installed marketplace plugin"* ]]
  [[ "$output" != *"naming the relevant plugin or skill"* ]]
  [[ "$output" == *"Do not inspect target repository state, mutate files, start evals, or run unrelated shell commands."* ]]
  [[ "$output" != *"deep_tracing: true"* ]]
  [[ "$output" == *"deep_tracing: false"* ]]
  [[ "$output" == *"tracing:"*$'\n'"  enabled: false"* ]]
  [[ "$output" == *"do not use, mention, or rely on prior conversations"* ]]
  [[ "$output" == *"sandbox_mode: read-only"* ]]
  [[ "$output" == *"skip_git_repo_check: true"* ]]
  [[ "$output" == *"working_dir: \"$expected_eval_workspace\""* ]]
  [[ "$output" != *"working_dir: \"$ROOT/"* ]]
  [[ "$output" == *"load-harness-cases.cjs"* ]]
}

@test "generated behavior config keeps all eval runtime state outside dependencies" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"$ROOT/.evals/codex-home-full-marketplace"* && "$output" != *"$ROOT/.dependencies/evals/"* ]]
}

@test "generated config uses local Codex auth for providers and graders" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" != *"apiKeyRequired: false"* ]]
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

@test "generated behavior config uses runtime loader when case filter is set" {
  run env EVAL_CASE_FILTER=tiber node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"evals/out/generated/load-harness-cases.runtime.cjs"* ]]
  [[ "$output" != *"tests: file://$ROOT/evals/promptfoo/load-harness-cases.cjs"* ]]
}

@test "forced skill invocation is an opt-in diagnostic without a no-plugin composition" {
  metadata="$(mktemp)"

  run env EVAL_SKILL_INVOCATION_MODE=forced \
    node "$GENERATOR" \
    --suite behavior \
    --metadata-output "$metadata" \
    --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"targeted-plugins"* ]]
  [[ "$output" == *"full-marketplace"* ]]
  [[ "$output" != *"no-plugins"* ]]
  [[ "$output" == *"skillInvocationMode: forced"* ]]
  [ "$(jq -r '.skillInvocationMode' "$metadata")" = "forced" ]
  [ "$(jq '[.providerCompositions[] | select(.pluginMode == "no-plugins")] | length' "$metadata")" = "0" ]

  rm -f "$metadata"

  run env \
    EVAL_SKILL_INVOCATION_MODE=forced \
    EVAL_PROVIDER_FILTER=no-plugins \
    node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 2 ]
  [[ "$output" == *"forced skill invocation cannot run against no-plugins"* ]]

  run env EVAL_SKILL_INVOCATION_MODE=forced \
    node "$GENERATOR" --suite canary --stdout

  [ "$status" -eq 2 ]
  [[ "$output" == *"forced skill invocation is available only for behavior evals"* ]]
}

@test "forced config generation validates exact fixture skill ownership" {
  make_codex_eval_fixture
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {
    "case_id": "forced-case",
    "prompt": "Answer the scenario.",
    "plugins": ["shared"],
    "skills": []
  }
]
JSON

  run env EVAL_SKILL_INVOCATION_MODE=forced \
    node "$FIXTURE_TMP/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 2 ]
  [[ "$output" == *"must declare a non-empty skills array"* ]]

  jq '.[0].skills = ["missing-skill"]' \
    "$FIXTURE_TMP/evals/fixtures/behavior/cases.json" \
    >"$FIXTURE_TMP/cases.updated.json"
  mv "$FIXTURE_TMP/cases.updated.json" \
    "$FIXTURE_TMP/evals/fixtures/behavior/cases.json"

  run env EVAL_SKILL_INVOCATION_MODE=forced \
    node "$FIXTURE_TMP/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 2 ]
  [[ "$output" == *"must resolve to exactly one declared plugin; found 0"* ]]

  mkdir -p "$FIXTURE_TMP/plugins/codex-only/skills/shared-skill"
  cp "$FIXTURE_TMP/plugins/shared/skills/shared-skill/SKILL.md" \
    "$FIXTURE_TMP/plugins/codex-only/skills/shared-skill/SKILL.md"
  jq '.[0].plugins = ["shared", "codex-only"] | .[0].skills = ["shared-skill"]' \
    "$FIXTURE_TMP/evals/fixtures/behavior/cases.json" \
    >"$FIXTURE_TMP/cases.updated.json"
  mv "$FIXTURE_TMP/cases.updated.json" \
    "$FIXTURE_TMP/evals/fixtures/behavior/cases.json"

  run env EVAL_SKILL_INVOCATION_MODE=forced \
    node "$FIXTURE_TMP/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 2 ]
  [[ "$output" == *"must resolve to exactly one declared plugin; found 2"* ]]
}

@test "generated targeted config includes a selected Codex plugin" {
  make_codex_eval_fixture
  cat >"$FIXTURE_TMP/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {
    "case_id": "codex-only-case",
    "plugins": ["codex-only"]
  }
]
JSON

  run env EVAL_CASE_FILTER=codex-only-case node "$FIXTURE_TMP/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"codex-gpt-5.6-terra-targeted-plugins"* ]]
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

@test "full marketplace canary assertion uses the Codex marketplace" {
  make_codex_eval_fixture

  run node - "$FIXTURE_TMP" <<'NODE'
const path = require('path');
process.chdir(process.argv[2]);
const assertCanary = require(path.join(process.argv[2], 'evals/promptfoo/assert-full-marketplace-canary.cjs'));

const codexMissingResult = assertCanary('Shared: Shared Skill');
if (codexMissingResult.pass !== false || !codexMissingResult.reason.includes('codex-only')) {
  throw new Error(`expected Codex canary to require Codex-only plugin: ${JSON.stringify(codexMissingResult)}`);
}

const codexResult = assertCanary('Shared: Shared Skill\nCodex Only: Codex Skill');
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
  'development-system',
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
  'Development System: Agentic Systems',
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

@test "codex eval home preparation refreshes stale seeded auth" {
  FIXTURE_TMP="$(mktemp -d)"
  auth_home="$FIXTURE_TMP/auth-source"
  eval_home="$FIXTURE_TMP/eval-home"
  mkdir -p "$auth_home" "$eval_home"
  printf '%s\n' '{"token":"current"}' >"$auth_home/auth.json"
  printf 'ai-plugins Codex eval home\n' >"$eval_home/.ai-plugins-eval-home"
  printf '%s\n' '{"token":"revoked"}' >"$eval_home/auth.json"

  run env -u OPENAI_API_KEY CODEX_EVAL_AUTH_HOME="$auth_home" node "$ROOT/scripts/evals/prepare-codex-home.mjs" "$eval_home" --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  cmp "$auth_home/auth.json" "$eval_home/auth.json"
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
