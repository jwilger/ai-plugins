#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "marketplace canary is discovery-only and does not cue known names" {
  run node - "$ROOT" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');

const root = process.argv[2];
const generateTests = require(path.join(root, 'evals/promptfoo/load-canary-cases.cjs'));
const tests = generateTests();
const canary = tests.find((testCase) => testCase.description === 'full-marketplace-canary');
const prompt = canary?.vars?.scenario_prompt || '';

if (!prompt.includes('discovery-only')) {
  throw new Error('canary must identify itself as discovery-only');
}
if (!prompt.includes('Do not modify files, invoke skills or tools, or perform workflow steps.')) {
  throw new Error('canary must prohibit behavior execution');
}

const marketplaceFiles = ['.agents/plugins/marketplace.json'];
const publicNames = new Set();
for (const relativeFile of marketplaceFiles) {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, relativeFile), 'utf8'));
  for (const plugin of manifest.plugins || []) publicNames.add(plugin.name);
}
for (const skill of fs.readdirSync(path.join(root, 'plugins/development-system/skills'))) {
  publicNames.add(skill);
}

const normalizedPrompt = prompt.toLowerCase();
for (const publicName of publicNames) {
  if (normalizedPrompt.includes(publicName.toLowerCase())) {
    throw new Error(`canary cues public name: ${publicName}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "eval artifacts use the public development-system suite label" {
  status_file="$TMPROOT/status.json"

  run node "$ROOT/scripts/evals/write-status.mjs" \
    --output "$status_file" \
    --state completed

  [ "$status" -eq 0 ]
  [ "$(jq -r '.suite' "$status_file")" = "development-system" ]


}

@test "eval status records the skill invocation mode" {
  status_file="$TMPROOT/status.json"

  run node "$ROOT/scripts/evals/write-status.mjs" \
    --output "$status_file" \
    --state completed \
    --skill-invocation-mode forced

  [ "$status" -eq 0 ]
  [ "$(jq -r '.skillInvocationMode' "$status_file")" = "forced" ]
}

