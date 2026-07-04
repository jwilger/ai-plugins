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
      const testCase = result.testCase?.vars || result.testCase || result.vars || {};
      const grading = result.gradingResult || result;
      const pass = Boolean(grading.pass ?? result.success ?? result.pass);
      const provider =
        result.provider?.label ||
        result.provider?.id ||
        result.provider ||
        result.prompt?.provider ||
        'unknown-provider';
      return {
        id: testCase.case_id || result.description || `case-${index + 1}`,
        behavior: testCase.behavior || '',
        provider,
        plugins: normalizeList(testCase.plugins || testCase.plugin),
        skills: normalizeList(testCase.skills || testCase.skill),
        sampleIndex: Number(testCase.sample_index ?? 1),
        minPassRate: Number(testCase.min_pass_rate ?? testCase.minPassRate ?? 1),
        pass,
        score: Number(grading.score ?? (pass ? 1 : 0)),
        reason: grading.reason || result.reason || '',
      };
    });
  }

  return [];
}

function normalizeList(value) {
  if (Array.isArray(value)) {
    return value.map(String).filter(Boolean);
  }

  if (!value) {
    return [];
  }

  if (typeof value === 'string' && value.trim().startsWith('[')) {
    try {
      const parsed = JSON.parse(value);
      if (Array.isArray(parsed)) {
        return parsed.map(String).filter(Boolean);
      }
    } catch {
      return [];
    }
  }

  return String(value)
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

function aggregateCases(cases) {
  const groups = new Map();

  for (const testCase of cases) {
    const key = `${testCase.provider}::${testCase.id}`;

    if (!groups.has(key)) {
      groups.set(key, {
        id: testCase.id,
        behavior: testCase.behavior,
        provider: testCase.provider,
        minPassRate: testCase.minPassRate,
        total: 0,
        passed: 0,
        failed: 0,
        passRate: 0,
        thresholdMet: false,
        samples: [],
      });
    }

    const group = groups.get(key);
    group.total += 1;
    group.passed += testCase.pass ? 1 : 0;
    group.failed += testCase.pass ? 0 : 1;
    group.minPassRate = Math.max(group.minPassRate, testCase.minPassRate);
    group.samples.push({
      sampleIndex: testCase.sampleIndex,
      pass: testCase.pass,
      score: testCase.score,
      reason: testCase.reason,
    });
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      passRate: group.total === 0 ? 0 : group.passed / group.total,
      thresholdMet:
        group.total > 0 &&
        group.passed / group.total >= group.minPassRate,
    }))
    .sort((left, right) =>
      `${left.provider}:${left.id}`.localeCompare(`${right.provider}:${right.id}`),
    );
}

function aggregateDimension(cases, field, idName) {
  const groups = new Map();

  for (const testCase of cases) {
    for (const id of testCase[field]) {
      const key = `${testCase.provider}::${id}`;

      if (!groups.has(key)) {
        groups.set(key, {
          [idName]: id,
          provider: testCase.provider,
          total: 0,
          passed: 0,
          failed: 0,
          passRate: 0,
          cases: new Set(),
        });
      }

      const group = groups.get(key);
      group.total += 1;
      group.passed += testCase.pass ? 1 : 0;
      group.failed += testCase.pass ? 0 : 1;
      group.cases.add(testCase.id);
    }
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      passRate: group.total === 0 ? 0 : group.passed / group.total,
      cases: [...group.cases].sort(),
    }))
    .sort((left, right) =>
      `${left.provider}:${left[idName]}`.localeCompare(
        `${right.provider}:${right[idName]}`,
      ),
    );
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
const aggregates = aggregateCases(cases);
const pluginSummaries = aggregateDimension(cases, 'plugins', 'plugin');
const skillSummaries = aggregateDimension(cases, 'skills', 'skill');
const passed = cases.filter((testCase) => testCase.pass).length;
const failed = cases.length - passed;
const summary = {
  generatedAt: new Date().toISOString(),
  suite: 'agentic-systems-engineering',
  total: cases.length,
  passed,
  failed,
  passRate: cases.length === 0 ? 0 : passed / cases.length,
  thresholdsMet: aggregates.filter((group) => group.thresholdMet).length,
  thresholdsFailed: aggregates.filter((group) => !group.thresholdMet).length,
  artifacts: {
    json: '../../evals/out/results.json',
    html: '../../evals/out/report.html',
    junit: '../../evals/out/results.junit.xml',
  },
  aggregates,
  pluginSummaries,
  skillSummaries,
  cases,
};

fs.writeFileSync(
  path.join(siteDir, 'summary.json'),
  `${JSON.stringify(summary, null, 2)}\n`,
);

const rows = aggregates
  .map(
    (testCase) => `<tr>
  <td>${escapeHtml(testCase.provider)}</td>
  <td>${escapeHtml(testCase.id)}</td>
  <td>${testCase.thresholdMet ? 'pass' : 'fail'}</td>
  <td>${(testCase.passRate * 100).toFixed(1)}% / ${(testCase.minPassRate * 100).toFixed(1)}%</td>
  <td>${escapeHtml(testCase.behavior)}</td>
  <td>${escapeHtml(testCase.samples.map((sample) => `#${sample.sampleIndex}: ${sample.reason}`).join(' | '))}</td>
</tr>`,
  )
  .join('\n');

function summaryRows(items, idName) {
  return items
    .map(
      (item) => `<tr>
  <td>${escapeHtml(item.provider)}</td>
  <td>${escapeHtml(item[idName])}</td>
  <td>${item.passed} / ${item.total}</td>
  <td>${(item.passRate * 100).toFixed(1)}%</td>
  <td>${escapeHtml(item.cases.join(', '))}</td>
</tr>`,
    )
    .join('\n');
}

const pluginRows = summaryRows(pluginSummaries, 'plugin');
const skillRows = summaryRows(skillSummaries, 'skill');

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
      <div class="metric"><strong>Thresholds met</strong><br>${summary.thresholdsMet}</div>
      <div class="metric"><strong>Thresholds failed</strong><br>${summary.thresholdsFailed}</div>
    </section>
    <table>
      <thead>
        <tr><th>Provider</th><th>Case</th><th>Status</th><th>Rate / threshold</th><th>Behavior</th><th>Samples</th></tr>
      </thead>
      <tbody>
${rows}
      </tbody>
    </table>
    <h2>Plugin summary</h2>
    <table>
      <thead>
        <tr><th>Provider</th><th>Plugin</th><th>Passed</th><th>Pass rate</th><th>Cases</th></tr>
      </thead>
      <tbody>
${pluginRows}
      </tbody>
    </table>
    <h2>Skill summary</h2>
    <table>
      <thead>
        <tr><th>Provider</th><th>Skill</th><th>Passed</th><th>Pass rate</th><th>Cases</th></tr>
      </thead>
      <tbody>
${skillRows}
      </tbody>
    </table>
  </main>
</body>
</html>
`;

fs.writeFileSync(path.join(siteDir, 'index.html'), html);
console.log(`wrote ${path.relative(root, siteDir)}/index.html`);
