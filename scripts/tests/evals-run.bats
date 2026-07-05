#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run.sh"
}

@test "eval runner prints help" {
  run "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage: scripts/evals/run.sh"* ]]
  [[ "$output" == *"Claude Code: provider=anthropic:claude-agent-sdk, model=sonnet, skills=all"* ]]
  [[ "$output" == *"Codex:       provider=openai:codex-sdk, model=gpt-5.5, model_reasoning_effort=medium"* ]]
  [[ "$output" == *"Each provider loads the relevant marketplace surface for its harness"* ]]
  [[ "$output" == *"Pinned eval packages are managed by package.json and package-lock.json"* ]]
  [[ "$output" == *"@openai/codex-sdk"* ]]
  [[ "$output" == *"@anthropic-ai/claude-agent-sdk"* ]]
  [[ "$output" == *"Requires working Claude Code and Codex model authentication"* ]]
  [[ "$output" == *"Prompt response caching and hosted sharing are disabled"* ]]
  [[ "$output" == *"results.junit.xml"* ]]
}

@test "eval runner dry-run uses provider-backed harness config and repo-owned artifacts" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/evals/ensure-node-deps.sh"* ]]
  [[ "$output" == *"node_modules/.bin/promptfoo"* ]]
  [[ "$output" != *"npx --yes"* ]]
  [[ "$output" == *"--max-concurrency 2"* ]]
  [[ "$output" == *"--no-cache"* ]]
  [[ "$output" == *"--no-share"* ]]
  [[ "$output" == *"evals/out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [[ "$output" == *"evals/out/results.json"* ]]
  [[ "$output" == *"evals/out/report.html"* ]]
  [[ "$output" == *"evals/out/results.junit.xml"* ]]
}

@test "eval runner dry-run prepares targeted Codex home from Codex marketplace plugins" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  targeted_line="$(printf '%s\n' "$output" | grep -- '--plugin-mode targeted-plugins')"
  [[ "$targeted_line" == *"prepare-codex-home.mjs"* ]]
  [[ "$targeted_line" == *"--plugins"* ]]
  [[ "$targeted_line" == *"\\,advisor"* || "$targeted_line" == *"advisor\\,"* || "$targeted_line" == *"--plugins advisor"* ]]
}

@test "eval runner fails when Codex marketplace has no plugin names" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/.agents/plugins"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/.agents/plugins/marketplace.json" <<'JSON'
{
  "plugins": []
}
JSON

  run "$fixture_root/scripts/evals/run.sh" --dry-run

  [ "$status" -ne 0 ]
  [[ "$output" == *"Codex marketplace has no plugins"* ]]
}

@test "package manifest pins promptfoo and coding harness provider SDKs" {
  run node - "$ROOT/package.json" <<'NODE'
const fs = require('fs');

const pkg = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const deps = pkg.devDependencies || {};
const expected = {
  promptfoo: '0.121.17',
  '@openai/codex-sdk': '0.142.5',
  '@anthropic-ai/claude-agent-sdk': '0.3.201',
};

for (const [name, version] of Object.entries(expected)) {
  if (deps[name] !== version) {
    throw new Error(`${name} should be pinned to ${version}, got ${deps[name] || 'missing'}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}
