const fs = require('fs');
const path = require('path');

module.exports = function generateTests() {
  const file = path.resolve(
    process.cwd(),
    'evals/fixtures/agentic-systems-engineering/cases.json',
  );
  const cases = JSON.parse(fs.readFileSync(file, 'utf8'));

  return cases.map((testCase) => ({
    description: testCase.case_id,
    vars: {
      case_id: testCase.case_id,
      behavior: testCase.behavior,
    },
    options: {
      provider: {
        id: 'local-skill-content',
      },
    },
  }));
};
