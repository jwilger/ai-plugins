#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  FIXTURE_ROOT="$(mktemp -d)"
  mkdir -p "$FIXTURE_ROOT/evals/fixtures/behavior"
  cat >"$FIXTURE_ROOT/evals/fixtures/behavior/cases.json" <<'JSON'
[
  {"case_id":"alpha","plugins":["alpha-plugin"],"valueGate":{"mode":"none"},"providerEval":{"unresolvedStochasticQuestion":"Will alpha be selected?","deterministicVerificationFullyDecides":false,"deterministicInsufficiency":"Provider selection remains probabilistic.","evidencePurpose":"absolute-reliability","baselineAuditDisposition":"Retain as an absolute reliability selection case; baseline lift is not the hypothesis."}},
  {"case_id":"beta","plugins":["beta-plugin"],"valueGate":{"mode":"none"},"providerEval":{"unresolvedStochasticQuestion":"Will beta be selected?","deterministicVerificationFullyDecides":false,"deterministicInsufficiency":"Provider selection remains probabilistic.","evidencePurpose":"absolute-reliability","baselineAuditDisposition":"Retain as an absolute reliability selection case; baseline lift is not the hypothesis."}},
  {"case_id":"alphabet","plugins":["alphabet-plugin"],"valueGate":{"mode":"none"},"providerEval":{"unresolvedStochasticQuestion":"Will alphabet be selected?","deterministicVerificationFullyDecides":false,"deterministicInsufficiency":"Provider selection remains probabilistic.","evidencePurpose":"absolute-reliability","baselineAuditDisposition":"Retain as an absolute reliability selection case; baseline lift is not the hypothesis."}}
]
JSON
}

teardown() {
  rm -rf "$FIXTURE_ROOT"
}

@test "behavior case filter supports anchored multi-case regular expressions" {
  run node - "$ROOT" "$FIXTURE_ROOT" <<'NODE'
const path = require('node:path');
const root = process.argv[2];
const fixtureRoot = process.argv[3];
const { selectedBehaviorCases } = require(path.join(root, 'evals/promptfoo/fixtures.cjs'));

const selected = selectedBehaviorCases({
  root: fixtureRoot,
  caseFilter: '^(alpha|beta)$',
});
const actual = selected.map((testCase) => testCase.case_id);
const expected = ['alpha', 'beta'];

if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  throw new Error(`${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "behavior case filter reports invalid regular expressions clearly" {
  run node - "$ROOT" "$FIXTURE_ROOT" <<'NODE'
const path = require('node:path');
const root = process.argv[2];
const fixtureRoot = process.argv[3];
const { selectedBehaviorCases } = require(path.join(root, 'evals/promptfoo/fixtures.cjs'));

try {
  selectedBehaviorCases({ root: fixtureRoot, caseFilter: '(' });
  throw new Error('invalid regular expression was accepted');
} catch (error) {
  if (!String(error.message).includes('invalid behavior case filter regex')) {
    throw error;
  }
}
NODE

  [ "$status" -eq 0 ]
}
