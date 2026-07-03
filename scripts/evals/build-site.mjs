#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '../..');
const outDir = path.join(root, 'evals/out');
const siteDir = path.join(root, 'site/evals');
const resultsPath = path.join(outDir, 'results.json');

function readResults(file) {
  const raw = JSON.parse(fs.readFileSync(file, 'utf8'));
  const results = raw.results?.results || raw.results || raw.prompts || [];

  if (Array.isArray(results)) {
    return results.map((result, index) => {
      const testCase = result.testCase || result.vars || {};
      const grading = result.gradingResult || result;
      const pass = Boolean(grading.pass ?? result.success ?? result.pass);
      return {
        id: testCase.case_id || result.description || `case-${index + 1}`,
        behavior: testCase.behavior || '',
        pass,
        score: Number(grading.score ?? (pass ? 1 : 0)),
        reason: grading.reason || result.reason || '',
      };
    });
  }

  return [];
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

fs.mkdirSync(siteDir, { recursive: true });

const cases = fs.existsSync(resultsPath) ? readResults(resultsPath) : [];
const passed = cases.filter((testCase) => testCase.pass).length;
const failed = cases.length - passed;
const summary = {
  generatedAt: new Date().toISOString(),
  suite: 'agentic-systems-engineering',
  total: cases.length,
  passed,
  failed,
  passRate: cases.length === 0 ? 0 : passed / cases.length,
  artifacts: {
    json: '../../evals/out/results.json',
    html: '../../evals/out/report.html',
    junit: '../../evals/out/results.junit.xml',
  },
  cases,
};

fs.writeFileSync(
  path.join(siteDir, 'summary.json'),
  `${JSON.stringify(summary, null, 2)}\n`,
);

const rows = cases
  .map(
    (testCase) => `<tr>
  <td>${escapeHtml(testCase.id)}</td>
  <td>${testCase.pass ? 'pass' : 'fail'}</td>
  <td>${escapeHtml(testCase.behavior)}</td>
  <td>${escapeHtml(testCase.reason)}</td>
</tr>`,
  )
  .join('\n');

const html = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ai-plugins eval dashboard</title>
  <style>
    body { font-family: ui-sans-serif, system-ui, sans-serif; margin: 2rem; color: #111827; }
    main { max-width: 72rem; margin: 0 auto; }
    table { width: 100%; border-collapse: collapse; margin-top: 1.5rem; }
    th, td { border-bottom: 1px solid #d1d5db; padding: 0.65rem; text-align: left; vertical-align: top; }
    th { background: #f3f4f6; }
    .summary { display: flex; gap: 1rem; flex-wrap: wrap; }
    .metric { border: 1px solid #d1d5db; padding: 0.75rem; min-width: 9rem; }
  </style>
</head>
<body>
  <main>
    <h1>ai-plugins eval dashboard</h1>
    <p>Suite: ${escapeHtml(summary.suite)}. Generated: ${escapeHtml(summary.generatedAt)}.</p>
    <section class="summary" aria-label="summary">
      <div class="metric"><strong>Total</strong><br>${summary.total}</div>
      <div class="metric"><strong>Passed</strong><br>${summary.passed}</div>
      <div class="metric"><strong>Failed</strong><br>${summary.failed}</div>
      <div class="metric"><strong>Pass rate</strong><br>${(summary.passRate * 100).toFixed(1)}%</div>
    </section>
    <table>
      <thead>
        <tr><th>Case</th><th>Status</th><th>Behavior</th><th>Reason</th></tr>
      </thead>
      <tbody>
${rows}
      </tbody>
    </table>
  </main>
</body>
</html>
`;

fs.writeFileSync(path.join(siteDir, 'index.html'), html);
console.log(`wrote ${path.relative(root, siteDir)}/index.html`);
