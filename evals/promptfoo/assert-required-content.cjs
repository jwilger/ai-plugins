const { fixtureFor } = require('./lib/cases.cjs');

module.exports = function assertRequiredContent(output, context) {
  const configured = fixtureFor(context?.vars?.case_id).mustContain || [];
  const required = Array.isArray(configured) ? configured : [configured];
  const missing = required.filter((needle) => !output.includes(needle));

  if (missing.length > 0) {
    return {
      pass: false,
      score: 0,
      reason: `Missing required content: ${missing.join(', ')}`,
    };
  }

  return {
    pass: true,
    score: 1,
    reason: 'All required content found',
  };
};
