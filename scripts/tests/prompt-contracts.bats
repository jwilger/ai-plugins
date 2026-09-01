#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "public docs route through canonical skill names" {
  grep -Fq '`development-system:agentic-systems`' "$ROOT/AGENTS.md"
  grep -Fq '`development-system:eval-case-reporting`' "$ROOT/AGENTS.md"
  grep -Fq '`development-system:engineering-standards`' "$ROOT/AGENTS.md"

  run rg -n \
    '`agentic-systems-engineering`|`eval-case-reporter`|/plugin install (agentic-systems-engineering|eval-case-reporter)' \
    "$ROOT/AGENTS.md" \
    "$ROOT/README.md"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
}

@test "active root prompts do not use retired public routing labels" {
  run rg -n \
    --pcre2 \
    '`agentic-systems-engineering`|`eval-case-reporter`|/plugin install (agentic-systems-engineering|eval-case-reporter)' \
    "$ROOT/plugins/development-system/skills" \
    "$ROOT/evals/fixtures/behavior"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
}

@test "behavior prompts do not cue retained component skill identities" {
  run rg -n \
    'development-discipline:|tiber:new-task' \
    "$ROOT/evals/fixtures/behavior"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
}

@test "active delivery prompts use the canonical direct-to-trunk mode" {
  run rg -n \
    'direct-to-main' \
    "$ROOT/AGENTS.md" \
    "$ROOT/README.md" \
    "$ROOT/plugins/development-system/skills" \
    "$ROOT/plugins/development-system/components/development-discipline/skills/development-workflow/SKILL.md" \
    "$ROOT/evals/fixtures/behavior"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
}

@test "active workflow names every executable lifecycle tool canonically" {
  workflow_skill="$ROOT/plugins/development-system/skills/development-workflow/SKILL.md"

  for tool in \
    workflow.record_red \
    workflow.authorize_implementation \
    workflow.record_green \
    workflow.begin_verification \
    workflow.record_verification \
    workflow.record_clean_review \
    workflow.authorize_delivery \
    workflow.complete_delivery; do
    grep -Fq "\`$tool\`" "$workflow_skill"
  done
}

@test "retained component identities are not public marketplace entries" {
  [ -d "$ROOT/plugins/development-system/components/agentic-systems-engineering" ]
  [ -d "$ROOT/plugins/development-system/components/eval-case-reporter" ]
  grep -Fq 'remain valid internal identities' "$ROOT/plugins/development-system/README.md"

  run rg -n \
    '"name": "(agentic-systems-engineering|eval-case-reporter)"' \
    "$ROOT/.agents/plugins/marketplace.json"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
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

  run rg -n 'suite: "agentic-systems-engineering"' \
    "$ROOT/scripts/evals/write-status.mjs" \
    "$ROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
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

@test "retired top-level eval decision and duplicate fixture stay removed" {
  [ ! -e "$ROOT/evals/fixtures/coverage-decisions.json" ]
  [ ! -e "$ROOT/evals/fixtures/agentic-systems-engineering/cases.json" ]
}

@test "behavior eval prefix does not cue plugin selection or answer provenance" {
  run rg -n \
    'Use installed marketplace plugin|name the relevant plugin|name the relevant.*skill' \
    "$ROOT/scripts/evals/generate-config.mjs"

  [ "$status" -eq 1 ]
  [ -z "$output" ]
}
