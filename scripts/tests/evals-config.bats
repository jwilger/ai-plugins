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
  [[ "$output" == *"Use installed marketplace plugin and skill guidance when it is relevant"* ]]
  [[ "$output" == *"When plugin or skill guidance documents a command, include the exact command name and flags instead of generic setup-path wording."* ]]
  [[ "$output" == *"You may read installed skill instruction files through the harness."* ]]
  [[ "$output" == *"Do not inspect target repository state, mutate files, start evals, or run unrelated shell commands."* ]]
  [[ "$output" != *"deep_tracing: true"* ]]
  [[ "$output" == *"deep_tracing: false"* ]]
  [[ "$output" == *"tracing:"*$'\n'"  enabled: false"* ]]
  [[ "$output" == *"Treat each scenario as stateless"* ]]
  [[ "$output" == *"sandbox_mode: read-only"* ]]
  [[ "$output" == *"skip_git_repo_check: true"* ]]
  [[ "$output" == *"codex_path_override: \"$ROOT/scripts/evals/codex-with-trusted-hooks.sh\""* ]]
  [[ "$output" == *"CODEX_EVAL_REAL_BIN:"* ]]
  [[ "$output" == *"working_dir: \"$ROOT/.evals/agent-workspace\""* ]]
  [[ "$output" == *"skills: all"* ]]
  [[ "$output" == *"setting_sources: []"* ]]
  [[ "$output" == *"persist_session: false"* ]]
  [[ "$output" == *"disallowed_tools:"*$'\n'"        - Bash"* ]]
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
  [[ "$output" == *"$ROOT/.evals/claude-home-development-system/plugin-cache/cache/ai-plugins/development-system/"* ]]
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
  [[ "$output" == *"CLAUDE_CONFIG_DIR: \"{{ env.CLAUDE_EVAL_RUNTIME_CONFIG_DIR_DEVELOPMENT_SYSTEM | default('$ROOT/.evals/claude-home-development-system/config') }}\""* ]]
  [[ "$output" == *"provider:"*$'\n'"      text:"*$'\n'"        id: openai:codex-sdk"* ]]
  [[ "$output" == *"CODEX_HOME: \"{{ env.CODEX_EVAL_HOME_DEVELOPMENT_SYSTEM | default(env.CODEX_EVAL_HOME)"* ]]
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

@test "generated Claude plugin path points at the isolated installed cache" {
  run node "$GENERATOR" --suite behavior --stdout

  [ "$status" -eq 0 ]
  version="$(jq -r '.version' "$ROOT/plugins/development-system/.claude-plugin/plugin.json")"
  [[ "$output" == *"path: \"{{ env.CLAUDE_EVAL_PLUGIN_PATH_DEVELOPMENT_SYSTEM"* ]]
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
