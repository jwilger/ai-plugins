const fs = require('fs');
const path = require('path');

const CASES_FILE = path.resolve(
  process.cwd(),
  'evals/fixtures/agentic-systems-engineering/cases.json',
);

function fileUrl(file) {
  return `file://${path.resolve(__dirname, file)}`;
}

module.exports = function generateTests() {
  const samples = Number.parseInt(process.env.EVAL_SAMPLES || '1', 10);
  const filter = process.env.EVAL_CASE_FILTER;
  const cases = JSON.parse(fs.readFileSync(CASES_FILE, 'utf8')).filter(
    (testCase) => !filter || testCase.case_id.includes(filter),
  );

  return cases.flatMap((testCase) =>
    Array.from({ length: samples }, (_, index) => ({
      description:
        samples > 1
          ? `${testCase.case_id} sample ${index + 1}`
          : testCase.case_id,
      vars: {
        case_id: testCase.case_id,
        behavior: testCase.behavior,
        scenario_prompt: testCase.prompt,
        sample_index: index + 1,
        min_pass_rate: testCase.minPassRate,
        plugins: testCase.plugins || [],
        skills: testCase.skills || [],
        tags: (testCase.tags || []).join(','),
      },
      assert: [
        {
          type: 'javascript',
          value: fileUrl('assert-hard-guards.cjs'),
        },
        {
          type: 'llm-rubric',
          value: testCase.semanticRubric,
        },
      ],
    })),
  );
};
