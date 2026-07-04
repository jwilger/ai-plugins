const fs = require('fs');
const path = require('path');

const CASES_FILE = path.resolve(
  process.cwd(),
  'evals/fixtures/agentic-systems-engineering/cases.json',
);

function fixtureFor(caseId) {
  const cases = JSON.parse(fs.readFileSync(CASES_FILE, 'utf8'));
  return cases.find((testCase) => testCase.case_id === caseId);
}

function isNegated(text) {
  return /\b(not|never|cannot|can[\u2019']?t|won[\u2019']?t|shouldn[\u2019']?t|wouldn[\u2019']?t|mustn[\u2019']?t|doesn[\u2019']?t|isn[\u2019']?t|aren[\u2019']?t|haven[\u2019']?t|hasn[\u2019']?t|wasn[\u2019']?t|without|refuse|decline|avoid|do not|don[\u2019']?t|should not|must not)\b/i.test(
    text,
  );
}

function forbiddenIntent(output, assertion) {
  const failures = [];

  for (const pattern of assertion.patterns || []) {
    const regex = new RegExp(pattern, 'gi');
    let match;

    while ((match = regex.exec(output)) !== null) {
      const before = output.slice(Math.max(0, match.index - 60), match.index);
      const evidence = `${before}${match[0]}`;

      if (!isNegated(evidence)) {
        failures.push(assertion.message || assertion.id);
      }

      if (match.index === regex.lastIndex) {
        regex.lastIndex += 1;
      }
    }
  }

  return failures;
}

module.exports = function assertHardGuards(output, context) {
  const testCase = fixtureFor(context?.vars?.case_id);

  if (!testCase) {
    return { pass: false, score: 0, reason: 'Unknown eval case' };
  }

  const failures = [];

  for (const assertion of testCase.hardAssertions || []) {
    if (assertion.type === 'forbiddenIntent') {
      failures.push(...forbiddenIntent(String(output || ''), assertion));
      continue;
    }

    failures.push(`Unsupported hard assertion type: ${assertion.type}`);
  }

  if (failures.length > 0) {
    return {
      pass: false,
      score: 0,
      reason: failures.join('; '),
    };
  }

  return {
    pass: true,
    score: 1,
    reason: 'Hard guard assertions passed',
  };
};
