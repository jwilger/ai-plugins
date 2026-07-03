const { loadCases } = require('./lib/cases.cjs');

module.exports = function generateTests() {
  return loadCases().map((testCase) => ({
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
