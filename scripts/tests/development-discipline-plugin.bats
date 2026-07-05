#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "development-discipline is registered for both harnesses with required skills and behavior fixtures" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const plugin = 'development-discipline';
const requiredSkills = [
  'test-driven-development',
  'verification-before-completion',
  'systematic-debugging',
  'receiving-code-review',
  'writing-skills',
];
const forbiddenSkills = [
  'using-superpowers',
  'subagent-driven-development',
  'dispatching-parallel-agents',
  'using-git-worktrees',
  'finishing-a-development-branch',
  'brainstorming',
];
const requiredCases = [
  'development-discipline-tdd-one-test-first',
  'development-discipline-verification-claim-scope',
  'development-discipline-review-feedback-skepticism',
  'development-discipline-skill-authoring-follows-marketplace',
  'development-discipline-systematic-debugging-root-cause',
];

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(root, relativePath), 'utf8'));
}

function fail(message) {
  failures.push(message);
}

const failures = [];
const pluginRoot = path.join(root, 'plugins', plugin);

if (!fs.existsSync(path.join(pluginRoot, '.claude-plugin', 'plugin.json'))) {
  fail('missing Claude plugin manifest');
}
if (!fs.existsSync(path.join(pluginRoot, '.codex-plugin', 'plugin.json'))) {
  fail('missing Codex plugin manifest');
}
if (!fs.existsSync(path.join(pluginRoot, 'README.md'))) {
  fail('missing plugin README');
}

for (const skill of requiredSkills) {
  if (!fs.existsSync(path.join(pluginRoot, 'skills', skill, 'SKILL.md'))) {
    fail(`missing required skill ${skill}`);
  }
}
for (const skill of forbiddenSkills) {
  if (fs.existsSync(path.join(pluginRoot, 'skills', skill, 'SKILL.md'))) {
    fail(`forbidden upstream workflow skill imported: ${skill}`);
  }
}

const claudeMarketplace = readJson('.claude-plugin/marketplace.json').plugins || [];
const codexMarketplace = readJson('.agents/plugins/marketplace.json').plugins || [];
if (!claudeMarketplace.some((entry) => entry.name === plugin && entry.source === `./plugins/${plugin}`)) {
  fail('missing Claude marketplace entry');
}
if (!codexMarketplace.some((entry) => entry.name === plugin && entry.source?.path === `./plugins/${plugin}`)) {
  fail('missing Codex marketplace entry');
}

const readme = fs.readFileSync(path.join(root, 'README.md'), 'utf8');
const readmeRow = `[${plugin}](plugins/${plugin}/README.md)`;
const readmeRowCount = readme.split(readmeRow).length - 1;
if (readmeRowCount !== 2) {
  fail(`expected README catalog row in both harness tables, found ${readmeRowCount}`);
}

const fixturesPath = path.join(root, 'evals/fixtures/behavior/full-marketplace/cases.json');
const cases = readJson('evals/fixtures/behavior/full-marketplace/cases.json');
for (const caseId of requiredCases) {
  const testCase = cases.find((entry) => entry.case_id === caseId);
  if (!testCase) {
    fail(`missing behavior fixture ${caseId}`);
    continue;
  }
  if (!testCase.plugins?.includes(plugin)) {
    fail(`${caseId}: missing ${plugin} plugin mapping`);
  }
  if (!Array.isArray(testCase.skills) || testCase.skills.length === 0) {
    fail(`${caseId}: missing skill mapping`);
  }
  if (typeof testCase.semanticRubric !== 'string' || testCase.semanticRubric.length < 80) {
    fail(`${caseId}: missing semantic rubric`);
  }
  if (!Array.isArray(testCase.calibration?.pass) || !Array.isArray(testCase.calibration?.fail)) {
    fail(`${caseId}: missing pass/fail calibration examples`);
  }
}

if (failures.length > 0) {
  console.error(`${fixturesPath}\n${failures.join('\n')}`);
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}
