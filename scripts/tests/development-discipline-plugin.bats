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
  'change-preflight',
  'delivery-workflow',
  'ci-failure-follow-up',
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
  'development-discipline-ci-failure-follow-up',
  'development-discipline-ci-failure-recovery-record',
  'development-discipline-rationale-bearing-commit-message',
  'development-discipline-delivery-direct-to-trunk',
  'development-discipline-delivery-pull-request',
  'development-discipline-delivery-local-only',
  'development-discipline-delivery-rejects-specialist-conflict',
  'development-discipline-delivery-current-user-restriction-wins',
  'development-discipline-delivery-composes-with-final-review',
  'development-discipline-preflight-feature',
  'development-discipline-preflight-bugfix',
  'development-discipline-preflight-refactor',
  'development-discipline-preflight-documentation-configuration',
  'development-discipline-preflight-packaging-release',
  'development-discipline-preflight-migration',
  'development-discipline-preflight-operational-change',
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
const preflightSkill = fs.readFileSync(
  path.join(pluginRoot, 'skills', 'change-preflight', 'SKILL.md'),
  'utf8',
);
const preflightDescription = preflightSkill.match(/^description:\s*(.+)$/m)?.[1] || '';
for (const trigger of ['documentation', 'configuration', 'packaging', 'release']) {
  if (!preflightDescription.toLowerCase().includes(trigger)) {
    fail(`change-preflight description missing trigger: ${trigger}`);
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
const localOnlyCase = cases.find(
  (entry) => entry.case_id === 'development-discipline-delivery-local-only',
);
const specialistConflictCase = cases.find(
  (entry) => entry.case_id === 'development-discipline-delivery-rejects-specialist-conflict',
);

if (!localOnlyCase?.semanticRubric.includes('describes proportionate local tests and review')) {
  fail('local-only rubric must assess the requested explanation, not claim work was performed');
}
if (
  !specialistConflictCase?.prompt.includes(
    'current user direction, repository-local instructions, the delivery-workflow router, then downstream specialist skills',
  )
) {
  fail('specialist-conflict prompt must request the exact chain graded by its rubric');
}
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

@test "development-discipline follows repository-local delivery policy" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8');
const normalize = (value) => value.toLowerCase().replace(/\s+/g, ' ');
const delivery = normalize(read('plugins/development-discipline/skills/delivery-workflow/SKILL.md'));
const finalReview = normalize(read('plugins/development-discipline/skills/final-review/SKILL.md'));
const standards = normalize(read('plugins/engineering-standards/skills/engineering-standards/SKILL.md'));
const workflowRule = normalize(read('docs/rules/workflow-and-commits.md'));
const developmentReadme = normalize(read('plugins/development-discipline/README.md'));
const standardsReadme = normalize(read('plugins/engineering-standards/README.md'));
const failures = [];

for (const phrase of [
  'repository-local instructions',
  'delivery-workflow router',
  'specialist skills',
  'direct-to-trunk',
  'pr/mr',
  'local-only',
  'do not invent a pull request',
  'externally visible',
  'destructive',
  'exact pushed revision',
  'current user direction comes first',
  'narrows standing repository authorization',
  'when repository policy requires ci',
  'changes the candidate revision',
  'rerun the repository-required checks and final review',
  'final review still applies in local-only mode',
  'local-only mode does not authorize a commit',
  'do not commit by default',
  'preserve repository-required branch or worktree topology',
  'do not ask again merely because an authorized action is first-time or consequential',
  'no exception can weaken that hold',
  "pr/mr's exact current head revision",
  'switching delivery modes cannot hide or bypass',
]) {
  if (!delivery.includes(phrase)) failures.push(`delivery skill missing: ${phrase}`);
}
for (const phrase of [
  'direct-to-trunk review before the first push',
  'local-only review',
  'do not require a pushed build',
  'final review applies even when repository policy forbids publishing',
]) {
  if (!finalReview.includes(phrase)) failures.push(`final-review skill missing: ${phrase}`);
}
for (const phrase of [
  'self-contained fallback',
  'repository-local policy is silent',
  'proportional to risk',
]) {
  if (!standards.includes(phrase)) failures.push(`engineering standards missing: ${phrase}`);
}
for (const [name, text] of [
  ['engineering standards', standards],
  ['canonical workflow rule', workflowRule],
  ['development-discipline README', developmentReadme],
  ['engineering-standards README', standardsReadme],
]) {
  if (!text.includes('repository-local')) failures.push(`${name} missing repository-local precedence`);
  if (!text.includes('delivery-workflow')) failures.push(`${name} missing delivery-workflow delegation`);
}
if (standards.includes('**pr-based**')) {
  failures.push('engineering standards still imposes PR-based delivery universally');
}
if (delivery.includes('without creating a feature branch')) {
  failures.push('direct-to-trunk must not forbid repository-required feature branches');
}
const scaffold = normalize(read('plugins/engineering-standards/skills/scaffold/SKILL.md'));
for (const phrase of [
  'commit only when the selected delivery policy authorizes or requires it',
  'use the commit cadence selected by repository-local delivery policy',
]) {
  if (!scaffold.includes(phrase)) {
    failures.push(`scaffold missing delivery-aware commit guidance: ${phrase}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "delivery-workflow benchmark rejects policy-invalid plans" {
  benchmark="$ROOT/plugins/development-discipline/skills/delivery-workflow/.plugin-eval/benchmark.json"
  workspace="$ROOT/plugins/development-discipline/skills/delivery-workflow/.plugin-eval/workspace"

  run jq -e '.verifiers.commands == ["node verify-delivery-plan.mjs"]' "$benchmark"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/direct-to-trunk-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/local-only-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/local-only-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/local-only-authorization-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/direct-to-trunk-invalid.json"
  [ "$status" -ne 0 ]
}

@test "development-discipline makes a failed pushed CI run a terminal-success hold" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const read = (relativePath) => {
  const target = path.join(root, relativePath);
  return fs.existsSync(target) ? fs.readFileSync(target, 'utf8') : '';
};
const normalize = (value) => value.toLowerCase().replace(/\s+/g, ' ');
const skill = normalize(
  read('plugins/development-discipline/skills/ci-failure-follow-up/SKILL.md'),
);
const tdd = normalize(read('plugins/development-discipline/skills/test-driven-development/SKILL.md'));
const finalReview = normalize(read('plugins/development-discipline/skills/final-review/SKILL.md'));
const rules = normalize(read('docs/rules/workflow-and-commits.md'));
const cases = JSON.parse(read('evals/fixtures/behavior/development-discipline/cases.json'));
const failures = [];

for (const phrase of [
  'failed commit sha',
  'run id or url',
  'exact failed job',
  'failed step',
  'relevant log evidence',
  'causal diagnosis',
  'supporting evidence',
  'unrelated implementation',
  'all pushes except the one recovery action',
  'next pushed commit',
  'unrelated or transient',
  'no intervening push',
  'never fold it into the active ticket',
  'failed rerun becomes the new failure record',
  'transition to action 1 in a separate recovery scope',
  'no unrelated commit is allowed',
  "active ticket's shared notes",
  'inspect the pushed ci runs for the active ticket',
  'failed run without a recorded terminal-success replacement',
  'even when a newer run is green or running',
  'terminal success',
  'queued|pending|running',
]) {
  if (!skill.includes(phrase)) failures.push(`CI follow-up skill missing: ${phrase}`);
}
for (const [name, text] of [
  ['TDD guidance', tdd],
  ['final-review guidance', finalReview],
  ['canonical workflow rule', rules],
]) {
  if (!text.includes('ci-failure-follow-up')) failures.push(`${name} missing skill delegation`);
}
for (const phrase of ['push only the diagnosed repair', 'rerun the unchanged revision', 'no intervening push']) {
  if (!rules.includes(phrase)) failures.push(`canonical workflow rule missing: ${phrase}`);
}
for (const phrase of ['no prior failed-run hold remains', 'newer running build does not mask']) {
  if (!finalReview.includes(phrase)) failures.push(`final-review entry gate missing: ${phrase}`);
}

const fixture = cases.find(
  (entry) => entry.case_id === 'development-discipline-ci-failure-follow-up',
);
if (!fixture) {
  failures.push('missing CI failure follow-up behavior fixture');
} else {
  if (!fixture.skills?.includes('ci-failure-follow-up')) {
    failures.push('CI failure follow-up fixture missing dedicated skill mapping');
  }
  const rubric = normalize(String(fixture.semanticRubric || ''));
  for (const phrase of [
    'exact failed job',
    'step',
    'next pushed commit',
    'unrelated or transient',
    'unchanged-revision rerun fails',
    'new failure record',
    'diagnosed, tested causal repair',
    'separate recovery scope',
    'shared ticket or handoff state',
    'pushed-run history',
    'newer build mask',
    'terminal success',
  ]) {
    if (!rubric.includes(phrase)) failures.push(`CI failure rubric missing: ${phrase}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
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
