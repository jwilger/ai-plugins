const fs = require('fs');
const path = require('path');

const CASES_FILE = path.resolve(
  process.cwd(),
  'evals/fixtures/agentic-systems-engineering/cases.json',
);

let cache;

function loadCases() {
  if (!cache) {
    cache = JSON.parse(fs.readFileSync(CASES_FILE, 'utf8'));
  }
  return cache;
}

function fixtureFor(caseId) {
  if (!caseId) return {};
  return loadCases().find((testCase) => testCase.case_id === caseId) || {};
}

module.exports = { fixtureFor, loadCases };
