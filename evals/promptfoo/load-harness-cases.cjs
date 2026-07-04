const path = require('path');
const {
  baselineLiftThreshold,
  coverageKinds,
  loadBehaviorCases,
  valueGateMode,
} = require('./fixtures.cjs');

const MATRIX_FILE = path.resolve(process.cwd(), 'evals/matrix.json');

function fileUrl(file) {
  return `file://${path.resolve(__dirname, file)}`;
}

function matrix() {
  try {
    return require(MATRIX_FILE);
  } catch {
    return {};
  }
}

function pluginModeFromEnvironment() {
  return process.env.EVAL_PLUGIN_MODE || 'full-marketplace';
}

module.exports = function generateTests() {
  const samples = Number.parseInt(process.env.EVAL_SAMPLES || '1', 10);
  const filter = process.env.EVAL_CASE_FILTER;
  const pluginMode = pluginModeFromEnvironment();
  const evalMatrix = matrix();
  const cases = loadBehaviorCases().filter(
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
        fixture_file: testCase.fixture_file,
        plugins: testCase.plugins || [],
        skills: testCase.skills || [],
        coverage_kinds: coverageKinds(testCase),
        value_gate_mode: valueGateMode(testCase),
        baseline_lift_threshold: baselineLiftThreshold(testCase, evalMatrix),
        plugin_mode: pluginMode,
        hard_guard_status:
          (testCase.hardAssertions || []).length > 0 ? 'configured' : 'none',
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
