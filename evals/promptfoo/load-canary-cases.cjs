const path = require('path');

function fileUrl(file) {
  return `file://${path.resolve(__dirname, file)}`;
}

module.exports = function generateCanaryTests() {
  return [
    {
      description: 'full-marketplace-canary',
      vars: {
        case_id: 'full-marketplace-canary',
        behavior: 'Proves the complete repository plugin marketplace is loaded.',
        scenario_prompt: [
          'This is a harness canary, not a behavior eval.',
          'Do not modify files.',
          'Do not inspect repository files or manifests; answer only from loaded plugin and skill context exposed by this harness.',
          'Use the loaded plugin and skill context available in this harness. If the agentic-systems-engineering skill is available, use it before answering.',
          'Reply with every repository marketplace plugin name that is available to this session and at least one representative skill or capability when available.',
        ].join('\n'),
        sample_index: 1,
      },
      assert: [
        {
          type: 'javascript',
          value: fileUrl('assert-full-marketplace-canary.cjs'),
        },
      ],
    },
  ];
};
