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
  'development-workflow',
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
  'development-workflow-normal-implementation',
  'development-workflow-ci-failure-hold',
  'development-workflow-pr-to-merge-readiness',
  'development-workflow-review-only-skips-implementation',
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
if (!preflightDescription.startsWith('Use when ')) {
  fail('change-preflight description must begin with Use when');
}
for (const trigger of ['documentation', 'configuration', 'packaging', 'release']) {
  if (!preflightDescription.toLowerCase().includes(trigger)) {
    fail(`change-preflight description missing trigger: ${trigger}`);
  }
}
if (!preflightDescription.includes('all ten surfaces')) {
  fail('change-preflight description must require all ten surfaces');
}
if (!preflightSkill.includes('Skip this skill when')) {
  fail('change-preflight must state an explicit skip condition');
}
if (!preflightSkill.includes('operational change | mixed')) {
  fail('change-preflight must use the canonical operational change classification');
}
if (!/Treat\s+those supplied facts as repository evidence/.test(preflightSkill)) {
  fail('change-preflight must explain evidence handling for advisory scenarios');
}
if (!preflightSkill.includes('Write all ten surface names explicitly')) {
  fail('change-preflight must require an explicit decision for every surface');
}
if (!preflightSkill.includes('Never substitute a generic checklist')) {
  fail('change-preflight must reject generic checklist output');
}
if (!/describes\s+runtime behavior does not itself change that behavior/.test(preflightSkill)) {
  fail('change-preflight must distinguish documentation from runtime behavior');
}
if (!preflightSkill.includes('deploy, observe, and recover')) {
  fail('change-preflight must include operator workflows');
}
if (!/compatibility, rollback,\s+recovery, and backfill/.test(preflightSkill)) {
  fail('change-preflight must require complete migration decisions');
}
if (!preflightSkill.includes('on-demand migration performed separately in each existing\nrepository as the backfill strategy')) {
  fail('change-preflight must treat per-repository on-demand migration as backfill');
}
if (!preflightSkill.includes('Do not defer a migration decision')) {
  fail('change-preflight must resolve migration rows before editing');
}
if (!preflightSkill.includes('Tie every row to a stated repository fact')) {
  fail('change-preflight must require row-level repository evidence');
}
if (!/chooses, copies, or applies\s+documented configuration/.test(preflightSkill)) {
  fail('change-preflight must include documented setup workflows');
}
for (const surface of ['Behavior:', 'Tests:', 'Documentation:', 'Configuration:', 'Packaging:', 'Release artifacts:', 'Migrations:', 'Operational startup:', 'Evaluations:', 'User workflows:']) {
  if (!preflightSkill.includes(surface)) {
    fail(`change-preflight output skeleton missing ${surface}`);
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
  if (
    caseId === 'development-discipline-preflight-feature' &&
    !testCase.semanticRubric.includes('directly maps a named repository artifact')
  ) {
    fail(`${caseId}: rubric must recognize row-level repository evidence`);
  }
  if (
    caseId === 'development-discipline-preflight-migration' &&
    (!testCase.semanticRubric.includes('backfill') ||
      !testCase.semanticRubric.includes('recovery') ||
      !testCase.semanticRubric.includes('on-demand migration is the backfill approach'))
  ) {
    fail(`${caseId}: rubric must enforce complete migration decisions`);
  }
  if (
    caseId === 'development-discipline-preflight-migration' &&
    !testCase.prompt.includes('no central batch backfill')
  ) {
    fail(`${caseId}: prompt must provide evidence for the backfill decision`);
  }
  if (!Array.isArray(testCase.calibration?.pass) || !Array.isArray(testCase.calibration?.fail)) {
    fail(`${caseId}: missing pass/fail calibration examples`);
  }
  if (
    (testCase.skills || []).includes('change-preflight') &&
    !testCase.prompt.includes('Use the repository facts stated here as evidence')
  ) {
    fail(`${caseId}: advisory prompt must identify its evidence source`);
  }
  if (
    (testCase.skills || []).includes('change-preflight') &&
    !testCase.prompt.startsWith('Use development-discipline:change-preflight.')
  ) {
    fail(`${caseId}: classification fixture must invoke the skill under test`);
  }
  if (
    (testCase.skills || []).includes('change-preflight') &&
    !testCase.prompt.includes("Follow the installed skill's exact output record")
  ) {
    fail(`${caseId}: classification fixture must require the skill contract`);
  }
  if (
    (testCase.skills || []).includes('change-preflight') &&
    !testCase.prompt.includes(
      'Use any installed skill content already supplied by the harness',
    )
  ) {
    fail(`${caseId}: classification fixture must require harness-supplied skill retrieval`);
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

@test "change-preflight benchmark rejects incomplete or speculative classifications" {
  benchmark="$ROOT/evals/benchmarks/change-preflight/benchmark.json"
  workspace="$ROOT/evals/benchmarks/change-preflight/workspace"
  test_support="$ROOT/evals/benchmarks/change-preflight/test-support"

  run jq -e '.verifiers.commands | length == 1' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e '.workspace.sourcePath == "evals/benchmarks/change-preflight/workspace"' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e '(.verifiers.commands[0] | contains("verify-change-preflight.mjs"))' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e 'any(.notes[]; contains("representative target remains unchanged"))' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e 'all(.scenarios[].userInput; (contains("Each evidence array") or contains("Their exact reasons")) | not)' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e 'all(.scenarios[].userInput; contains("surfaces object"))' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e '(.scenarios[] | select(.id == "feature") | .userInput) | contains("scenario exactly to `feature`")' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e '(.scenarios[] | select(.id == "docs-config") | .userInput) | contains("scenario exactly to `docs-config`")' "$benchmark"
  [ "$status" -eq 0 ]

  run jq -e 'any(.scenarios[] | select(.id == "docs-config") | .successChecklist[]; contains("Behavior is not applicable because runtime is unchanged"))' "$benchmark"
  [ "$status" -eq 0 ]

  run grep -F "The checked-in example is not loaded directly" "$workspace/docs-config/config/example.toml"
  [ "$status" -eq 0 ]

  run grep -F "Startup reads the user's copied configuration" "$workspace/docs-config/scripts/start.sh"
  [ "$status" -eq 0 ]

  run jq -e '(.scenarios[] | select(.id == "migration") | .userInput) | contains("scenario exactly to `migration`")' "$benchmark"
  [ "$status" -eq 0 ]

  for evidence in \
    AGENTS.md \
    feature/src/commands.md \
    feature/tests/cli.bats \
    docs-config/config/example.toml \
    migration/src/task-schema.md; do
    run test -f "$workspace/$evidence"
    [ "$status" -eq 0 ]
  done

  run test -f "$workspace/project/implementation-target.txt"
  [ "$status" -eq 0 ]

  changed_workspace="$(mktemp -d)"
  cp -R "$workspace/." "$changed_workspace/"
  printf '%s\n' 'implementation started early' >"$changed_workspace/project/implementation-target.txt"
  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/feature-valid.json" "$changed_workspace" feature
  [ "$status" -ne 0 ]

  changed_evidence_workspace="$(mktemp -d)"
  cp -R "$workspace/." "$changed_evidence_workspace/"
  printf '%s\n' 'implementation started early' >>"$changed_evidence_workspace/feature/request.md"
  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/feature-valid.json" "$changed_evidence_workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"representative repository changed before preflight completed"* ]]

  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/feature-valid.json" "$workspace" feature
  [ "$status" -eq 0 ]

  wrong_scenario="$(mktemp)"
  jq '.scenario = "docs-config"' "$test_support/fixtures/feature-valid.json" >"$wrong_scenario"
  run node "$workspace/verify-change-preflight.mjs" "$wrong_scenario" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"record does not match trusted scenario"* ]]

  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/docs-config-valid.json" "$workspace" docs-config
  [ "$status" -eq 0 ]

  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/migration-valid.json" "$workspace" migration
  [ "$status" -eq 0 ]

  live_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_workspace="$live_root/workspace"
  mkdir "$live_workspace"
  cp -R "$workspace/." "$live_workspace/"
  cp "$test_support/fixtures/feature-valid.json" "$live_workspace/change-preflight.json"
  verifier_command="$(jq -r '.verifiers.commands[0]' "$benchmark")"
  pushd "$live_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -eq 0 ]

  cross_case_root="$(mktemp -d -p /tmp plugin-eval-docs-config-XXXXXXXX)"
  cross_case_workspace="$cross_case_root/workspace"
  mkdir "$cross_case_workspace"
  cp -R "$workspace/." "$cross_case_workspace/"
  cp "$test_support/fixtures/feature-valid.json" "$cross_case_workspace/change-preflight.json"
  pushd "$cross_case_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  printf '%s\n' 'implementation started early' >"$live_workspace/project/implementation-target.txt"
  pushd "$live_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  live_plan_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_plan_workspace="$live_plan_root/workspace"
  mkdir "$live_plan_workspace"
  cp -R "$workspace/." "$live_plan_workspace/"
  jq '.surfaces.behavior.plan = ["edit source"]' "$test_support/fixtures/feature-valid.json" >"$live_plan_workspace/change-preflight.json"
  pushd "$live_plan_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  live_evidence_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_evidence_workspace="$live_evidence_root/workspace"
  mkdir "$live_evidence_workspace"
  cp -R "$workspace/." "$live_evidence_workspace/"
  cp "$test_support/fixtures/feature-valid.json" "$live_evidence_workspace/change-preflight.json"
  printf '%s\n' 'implementation started early' >>"$live_evidence_workspace/feature/request.md"
  pushd "$live_evidence_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  live_reason_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_reason_workspace="$live_reason_root/workspace"
  mkdir "$live_reason_workspace"
  cp -R "$workspace/." "$live_reason_workspace/"
  jq '.surfaces.behavior.reason = "Not applicable because behavior is unchanged."' "$test_support/fixtures/feature-valid.json" >"$live_reason_workspace/change-preflight.json"
  pushd "$live_reason_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  valid="$test_support/fixtures/feature-valid.json"
  invalid="$(mktemp)"

  jq 'del(.surfaces.evaluations)' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"all and only the ten required surfaces"* ]]

  jq '.surfaces.behavior = {status:"not-applicable", evidence:["feature/src/commands.md"], reason:"No behavior changes are requested."}' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior must be applicable for feature"* ]]

  jq '.implementationPlan = ["edit source"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"unexpected top-level fields: implementationPlan"* ]]

  jq '.surfaces.behavior.evidence = ["nearby code"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior evidence is not grounded"* ]]

  jq '.steps = []' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"unexpected top-level fields: steps"* ]]

  jq '.surfaces.configuration.evidence = ["feature/src/commands.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"configuration evidence is not grounded"* ]]

  jq '.surfaces.behavior.plan = ["edit source"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior has unexpected fields: plan"* ]]

  jq '.repositoryPolicyEvidence = ["repository policy"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"repositoryPolicyEvidence must cite repository facts"* ]]

  jq '.repositoryPolicyEvidence = ["README.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"repositoryPolicyEvidence must cite repository facts"* ]]

  jq '.surfaces.behavior.evidence = ["feature/request.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior evidence is not grounded"* ]]

  jq '.surfaces.configuration.reason = "Not applicable."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"configuration not-applicable decision needs a scenario-specific reason"* ]]

  jq '.surfaces.behavior.reason = "Not applicable because behavior is unchanged."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior has unexpected fields: reason"* ]]

  jq '.repositoryPolicyEvidence = ["AGENTS.md", "README.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"repositoryPolicyEvidence must cite repository facts"* ]]

  jq 'del(.surfaces.behavior.effect)' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.configuration.reason = "This unrelated sentence is comfortably long enough."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"scenario-specific reason"* ]]

  jq '.surfaces.behavior.evidence += ["feature/request.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior evidence is not grounded"* ]]

  jq '.surfaces.behavior.effect = "This sentence is long but says nothing useful."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.behavior.effect = "No command or behavior changes are needed."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.behavior.effect = "Runtime behavior remains unchanged."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.configuration.reason = "Runtime configuration defaults absolutely change."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"configuration not-applicable decision needs a scenario-specific reason"* ]]

  jq 'del(.surfaces.migrations.decisions.backfill)' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]

  jq '.surfaces.migrations.decisions.backfill = "This sentence is long but says nothing useful."' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]

  jq '.surfaces.migrations.decisions.compatibility = "Compatibility will be broken and old repositories stop working." | .surfaces.migrations.decisions.rollback = "Rollback is impossible and unsupported for operators."' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]

  jq '.surfaces.migrations.decisions.backfill = "Existing repositories will never receive backfill."' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]
}

@test "change-preflight keeps populated secrets outside preflight evidence" {
  skill="$ROOT/plugins/development-discipline/skills/change-preflight/SKILL.md"

  run grep -F "Never open, quote, hash, or include populated secret files" "$skill"
  [ "$status" -eq 0 ]

  run grep -F "populated secret files or secret values" "$skill"
  [ "$status" -eq 0 ]
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
