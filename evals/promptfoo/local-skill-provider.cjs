const fs = require('fs');
const path = require('path');

class LocalSkillProvider {
  id() {
    return 'local-skill-content';
  }

  async callApi(prompt, context) {
    const vars = caseVarsFromPrompt(prompt, context);
    const configuredFiles = vars.files || [];
    const files = Array.isArray(configuredFiles) ? configuredFiles : [configuredFiles];
    const output = files
      .map((file) => {
        const absolute = path.resolve(process.cwd(), file);
        const content = fs.readFileSync(absolute, 'utf8');
        return `--- ${file} ---\n${content}`;
      })
      .join('\n\n');

    return { output };
  }
}

function caseVarsFromPrompt(prompt, context) {
  const caseId =
    context?.vars?.case_id || String(prompt).match(/^Case ([^:]+):/)?.[1];
  if (!caseId) return {};

  const file = path.resolve(
    process.cwd(),
    'evals/fixtures/agentic-systems-engineering/cases.json',
  );
  const cases = JSON.parse(fs.readFileSync(file, 'utf8'));
  return cases.find((testCase) => testCase.case_id === caseId) || {};
}

module.exports = LocalSkillProvider;
