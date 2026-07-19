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
  'rationale-commit-messages',
  'verification-before-completion',
  'systematic-debugging',
  'receiving-code-review',
  'writing-skills',
  'final-review',
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
  'development-discipline-final-review-clean-iterations',
  'development-discipline-tdd-lightweight-post-implementation-review',
  'development-discipline-rationale-bearing-commit-message',
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

const fixturePaths = [
  'evals/fixtures/behavior/full-marketplace/cases.json',
  'evals/fixtures/behavior/development-discipline/cases.json',
];
const cases = fixturePaths.flatMap(readJson);
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
  console.error(`${fixturePaths.join('\n')}\n${failures.join('\n')}`);
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "development-discipline encodes the green increment and CI follow-up loop" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8');
const normalize = (value) => value.toLowerCase().replace(/\s+/g, ' ');
const tdd = normalize(read('plugins/development-discipline/skills/test-driven-development/SKILL.md'));
const finalReview = normalize(read('plugins/development-discipline/skills/final-review/SKILL.md'));
const cases = JSON.parse(read('evals/fixtures/behavior/development-discipline/cases.json'));
const failures = [];

for (const phrase of [
  'fast unit tests',
  'commit and push',
  'latest pushed build',
  'running or green',
  'failed build',
  'long-running',
  'full review',
]) {
  if (!tdd.includes(phrase)) failures.push(`TDD guidance missing: ${phrase}`);
}
for (const phrase of [
  'fast unit tests',
  'lightweight review',
  'commit and push',
  'latest pushed build',
  'delta risk assessment',
]) {
  if (!finalReview.includes(phrase)) failures.push(`final-review guidance missing: ${phrase}`);
}

const fixture = cases.find(
  (entry) => entry.case_id === 'development-discipline-green-increment-ci-loop',
);
if (!fixture) {
  failures.push('missing green-increment CI behavior fixture');
} else {
  const rubric = normalize(String(fixture.semanticRubric || ''));
  for (const phrase of [
    'fast unit tests',
    'lightweight review',
    'commit and push',
    'running or green',
    'failed',
    'full review',
  ]) {
    if (!rubric.includes(phrase)) failures.push(`green-increment fixture missing: ${phrase}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "development-discipline documents supported exceptional review evidence" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8');
const skill = read('plugins/development-discipline/skills/final-review/SKILL.md');
const protocol = read(
  'plugins/development-discipline/skills/final-review/references/mcp-protocol.md',
);
const cases = JSON.parse(read('evals/fixtures/behavior/development-discipline/cases.json'));
const failures = [];
const triggers = [
  'destructive-or-irreversible-operation',
  'authentication-or-authorization-boundary',
  'sensitive-data-migration',
  'cryptographic-behavior',
  'safety-critical-behavior',
];

for (const trigger of triggers) {
  if (!skill.includes(trigger)) failures.push(`final-review guidance missing: ${trigger}`);
  if (!protocol.includes(trigger)) failures.push(`MCP protocol missing: ${trigger}`);
}

const exceptional = cases.find(
  (entry) => entry.case_id === 'final-review-exceptional-risk-requires-supported-evidence',
);
if (!exceptional) {
  failures.push('missing exceptional-risk evidence behavior fixture');
} else {
  const rubric = String(exceptional.semanticRubric || '');
  for (const trigger of triggers) {
    if (!rubric.includes(trigger)) failures.push(`exceptional-risk fixture missing: ${trigger}`);
  }
  if (!rubric.includes('explicitly exceptional dimensions')) {
    failures.push('exceptional-risk fixture does not target second passes by dimension');
  }
}

const disposition = cases.find(
  (entry) => entry.case_id === 'final-review-routes-unrelated-findings-with-security-override',
);
if (!String(disposition?.semanticRubric || '').includes('do not pause')) {
  failures.push('risk-planned disposition fixture does not reject legacy confirmation pauses');
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "final-review separates review batching from delivery decomposition" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8');
const normalize = (value) => value.toLowerCase().replace(/\s+/g, ' ');
const skill = normalize(read('plugins/development-discipline/skills/final-review/SKILL.md'));
const protocol = normalize(
  read('plugins/development-discipline/skills/final-review/references/mcp-protocol.md'),
);
const cases = JSON.parse(read('evals/fixtures/behavior/development-discipline/cases.json'));
const failures = [];

for (const phrase of [
  'review_lifecycle',
  'propagates it through delta reassessment',
  'retrospective review',
  'delivery_boundaries',
  'final_review.confirm_split',
  'explicit user confirmation',
  'review-only branch',
  'blocking dependencies',
  'administrative review',
  'split_lineage',
  'generation one is the maximum',
]) {
  if (!skill.includes(phrase)) failures.push(`final-review guidance missing: ${phrase}`);
  if (!protocol.includes(phrase)) failures.push(`MCP protocol missing: ${phrase}`);
}

const fixture = cases.find(
  (entry) => entry.case_id === 'final-review-scope-growth-forces-ticket-split',
);
if (!fixture) {
  failures.push('missing scope-growth split behavior fixture');
} else {
  const rubric = normalize(String(fixture.semanticRubric || ''));
  for (const phrase of [
    'already landed',
    'retrospective',
    'build, test, and shipping',
    'explicit user confirmation',
    'review-only branches',
    'blocking dependencies',
    'recursive',
    'generation one is the maximum',
  ]) {
    if (!rubric.includes(phrase)) failures.push(`scope-growth fixture missing: ${phrase}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}
