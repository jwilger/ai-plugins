#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "../..");
const outDir = path.join(root, "evals/out");
const siteDir = path.join(root, "site/evals");
const resultsPath = path.join(outDir, "results.json");
const statusPath = path.join(outDir, "status.json");

function readResults(file) {
  const raw = JSON.parse(fs.readFileSync(file, "utf8"));
  const results = raw.results?.results || raw.results || raw.prompts || [];

  if (Array.isArray(results)) {
    return results.map((result, index) => {
      const testCase =
        result.testCase?.vars || result.testCase || result.vars || {};
      const grading = result.gradingResult || result;
      const pass = Boolean(grading.pass ?? result.success ?? result.pass);
      const reason =
        grading.reason ||
        result.reason ||
        grading.error ||
        result.error ||
        result.failureReason ||
        "";
      const blocked = isProviderUnavailable(reason);
      const provider =
        result.provider?.label ||
        result.provider?.id ||
        result.provider ||
        result.prompt?.provider ||
        "unknown-provider";
      const providerVariant =
        testCase.provider_variant ||
        testCase.providerVariant ||
        String(provider).replace(/-(no-plugins|targeted-plugins|full-marketplace)$/, "");
      const pluginMode =
        testCase.plugin_mode ||
        testCase.pluginMode ||
        String(provider).match(/(no-plugins|targeted-plugins|full-marketplace)$/)?.[1] ||
        "unknown";
      return {
        id: testCase.case_id || result.description || `case-${index + 1}`,
        behavior: testCase.behavior || "",
        provider,
        providerVariant,
        pluginMode,
        plugins: normalizeList(testCase.plugins || testCase.plugin),
        skills: normalizeList(testCase.skills || testCase.skill),
        sampleIndex: Number(testCase.sample_index ?? 1),
        minPassRate: Number(
          testCase.min_pass_rate ?? testCase.minPassRate ?? 1,
        ),
        valueGateMode:
          testCase.value_gate_mode || testCase.valueGateMode || "standard",
        baselineLiftThreshold: Number(
          testCase.baseline_lift_threshold ??
            testCase.baselineLiftThreshold ??
            0.1,
        ),
        hardGuardStatus:
          testCase.hard_guard_status || testCase.hardGuardStatus || "unknown",
        tokenUsage: result.tokenUsage || grading.tokenUsage || null,
        cost: Number(result.cost ?? grading.cost ?? 0),
        pass,
        blocked,
        status: pass ? "passed" : blocked ? "blocked" : "failed",
        score: Number(grading.score ?? (pass ? 1 : 0)),
        reason,
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

  if (typeof value === "string" && value.trim().startsWith("[")) {
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
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function isProviderUnavailable(reason) {
  const message = String(reason || "");

  return /\b(rate.?limit|weekly limit|session limit|usage limit|insufficient_quota|quota (?:exceeded|exhausted)|(?:exceeded|exhausted) quota|too many requests|429|provider unavailable|could not be resolved)\b/i.test(
    message,
  ) || /\b(?:Error calling (?:Claude Agent SDK|OpenAI Codex SDK)|provider error)\b[\s\S]{0,180}\b(?:not logged in|login required|authentication required|credentials? (?:missing|required)|api key (?:missing|required|not set)|unauthorized|forbidden|401|403)\b/i.test(
    message,
  ) || /\b(?:OPENAI_API_KEY|ANTHROPIC_API_KEY)\b[\s\S]{0,80}\b(?:missing|required|not set)\b/i.test(
    message,
  );
}

function aggregateCases(cases) {
  const groups = new Map();

  for (const testCase of cases) {
    const key = `${testCase.providerVariant}::${testCase.pluginMode}::${testCase.id}`;

    if (!groups.has(key)) {
      groups.set(key, {
        id: testCase.id,
        behavior: testCase.behavior,
        provider: testCase.provider,
        providerVariant: testCase.providerVariant,
        pluginMode: testCase.pluginMode,
        plugins: testCase.plugins,
        skills: testCase.skills,
        minPassRate: testCase.minPassRate,
        hardGuardStatus: testCase.hardGuardStatus,
        valueGateMode: testCase.valueGateMode,
        baselineLiftThreshold: testCase.baselineLiftThreshold,
        cost: 0,
        total: 0,
        passed: 0,
        failed: 0,
        blocked: 0,
        evaluated: 0,
        passRate: 0,
        thresholdMet: false,
        status: "failed",
        samples: [],
      });
    }

    const group = groups.get(key);
    group.total += 1;
    group.passed += testCase.pass ? 1 : 0;
    group.blocked += testCase.blocked ? 1 : 0;
    group.failed += !testCase.pass && !testCase.blocked ? 1 : 0;
    group.evaluated += testCase.blocked ? 0 : 1;
    group.minPassRate = Math.max(group.minPassRate, testCase.minPassRate);
    group.cost += testCase.cost || 0;
    group.samples.push({
      sampleIndex: testCase.sampleIndex,
      pass: testCase.pass,
      blocked: testCase.blocked,
      status: testCase.status,
      score: testCase.score,
      reason: testCase.reason,
      tokenUsage: testCase.tokenUsage,
    });
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      passRate: group.evaluated === 0 ? 0 : group.passed / group.evaluated,
      thresholdMet:
        group.evaluated > 0 &&
        group.passed / group.evaluated >= group.minPassRate,
      status:
        group.blocked > 0 && group.evaluated === 0
          ? "blocked"
          : group.blocked > 0
            ? "inconclusive"
            : group.evaluated > 0 &&
                group.passed / group.evaluated >= group.minPassRate
              ? "pass"
              : "fail",
    }))
    .sort((left, right) =>
      `${left.providerVariant}:${left.pluginMode}:${left.id}`.localeCompare(
        `${right.providerVariant}:${right.pluginMode}:${right.id}`,
      ),
    );
}

function aggregateDimension(cases, field, idName) {
  const groups = new Map();

  for (const testCase of cases) {
    for (const id of testCase[field]) {
      const key = `${testCase.providerVariant}::${testCase.pluginMode}::${id}`;

      if (!groups.has(key)) {
        groups.set(key, {
          [idName]: id,
          provider: testCase.provider,
          providerVariant: testCase.providerVariant,
          pluginMode: testCase.pluginMode,
          total: 0,
          passed: 0,
          failed: 0,
          blocked: 0,
          evaluated: 0,
          passRate: 0,
          cases: new Set(),
        });
      }

      const group = groups.get(key);
      group.total += 1;
      group.passed += testCase.pass ? 1 : 0;
      group.blocked += testCase.blocked ? 1 : 0;
      group.failed += !testCase.pass && !testCase.blocked ? 1 : 0;
      group.evaluated += testCase.blocked ? 0 : 1;
      group.cases.add(testCase.id);
    }
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      passRate: group.evaluated === 0 ? 0 : group.passed / group.evaluated,
      cases: [...group.cases].sort(),
    }))
    .sort((left, right) =>
      `${left.providerVariant}:${left.pluginMode}:${left[idName]}`.localeCompare(
        `${right.providerVariant}:${right.pluginMode}:${right[idName]}`,
      ),
    );
}

function valueGateSummaries(aggregates) {
  const byCase = new Map();
  for (const aggregate of aggregates) {
    const key = `${aggregate.providerVariant}::${aggregate.id}`;
    if (!byCase.has(key)) {
      byCase.set(key, []);
    }
    byCase.get(key).push(aggregate);
  }

  return [...byCase.entries()]
    .map(([key, groups]) => {
      const [providerVariant, caseId] = key.split("::");
      const full = groups.find((group) => group.pluginMode === "full-marketplace");
      const baseline = groups.find((group) => group.pluginMode === "no-plugins");
      const targeted = groups.find(
        (group) => group.pluginMode === "targeted-plugins",
      );
      const reference = full || targeted || groups[0];
      const fullComplete = full && full.evaluated > 0 && full.blocked === 0;
      const baselineComplete =
        baseline && baseline.evaluated > 0 && baseline.blocked === 0;
      const lift =
        fullComplete && baselineComplete
          ? full.passRate - baseline.passRate
          : null;
      const status =
        reference.valueGateMode === "safety-critical"
          ? fullComplete &&
            baselineComplete &&
            full.status === "pass" &&
            baseline.status !== "pass"
            ? "pass"
            : !fullComplete || !baselineComplete
              ? "unsupported"
              : "fail"
          : fullComplete &&
              baselineComplete &&
              full.status === "pass" &&
              lift >= reference.baselineLiftThreshold
            ? "pass"
            : fullComplete && !baselineComplete && full.status === "pass"
              ? "unsupported"
              : "fail";

      return {
        caseId,
        providerVariant,
        plugin: reference.plugins?.[0],
        skill: reference.skills?.[0],
        mode: reference.valueGateMode,
        baselineLiftThreshold: reference.baselineLiftThreshold,
        fullMarketplacePassRate: full?.passRate ?? null,
        noPluginsPassRate: baseline?.passRate ?? null,
        targetedPluginsPassRate: targeted?.passRate ?? null,
        lift,
        status,
      };
    })
    .sort((left, right) =>
      `${left.providerVariant}:${left.caseId}`.localeCompare(
        `${right.providerVariant}:${right.caseId}`,
      ),
    );
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function defaultStatus(cases) {
  if (cases.length > 0) {
    return {
      generatedAt: new Date().toISOString(),
      suite: "agentic-systems-engineering",
      state: "completed",
      reason: "Promptfoo results were found and summarized.",
      providerCredentials: "available",
    };
  }

  return {
    generatedAt: new Date().toISOString(),
    suite: "agentic-systems-engineering",
    state: "empty",
    reason: "No Promptfoo results were found.",
    providerCredentials: "unknown",
  };
}

function readStatus(file, cases) {
  if (!fs.existsSync(file)) {
    return defaultStatus(cases);
  }

  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    return defaultStatus(cases);
  }
}

fs.mkdirSync(siteDir, { recursive: true });

const cases = fs.existsSync(resultsPath) ? readResults(resultsPath) : [];
const runStatus = readStatus(statusPath, cases);
const aggregates = aggregateCases(cases);
const pluginSummaries = aggregateDimension(cases, "plugins", "plugin");
const skillSummaries = aggregateDimension(cases, "skills", "skill");
const valueGates = valueGateSummaries(aggregates);
const passed = cases.filter((testCase) => testCase.pass).length;
const blocked = cases.filter((testCase) => testCase.blocked).length;
const failed = cases.length - passed - blocked;
const evaluated = passed + failed;
const summary = {
  generatedAt: new Date().toISOString(),
  suite: "agentic-systems-engineering",
  status: runStatus,
  total: cases.length,
  passed,
  failed,
  blocked,
  passRate: evaluated === 0 ? 0 : passed / evaluated,
  thresholdsMet: aggregates.filter((group) => group.thresholdMet).length,
  thresholdsFailed: aggregates.filter((group) => group.status === "fail")
    .length,
  thresholdsBlocked: aggregates.filter((group) =>
    ["blocked", "inconclusive"].includes(group.status),
  ).length,
  valueGatesPassed: valueGates.filter((gate) => gate.status === "pass").length,
  valueGatesFailed: valueGates.filter((gate) => gate.status === "fail").length,
  artifacts: {
    json: "../../evals/out/results.json",
    html: "../../evals/out/report.html",
    junit: "../../evals/out/results.junit.xml",
  },
  aggregates,
  pluginSummaries,
  skillSummaries,
  valueGateSummaries: valueGates,
  cases,
};

fs.writeFileSync(
  path.join(siteDir, "summary.json"),
  `${JSON.stringify(summary, null, 2)}\n`,
);

const rows = aggregates
  .map(
    (testCase) => `<tr>
  <td>${escapeHtml(testCase.providerVariant)}</td>
  <td>${escapeHtml(testCase.pluginMode)}</td>
  <td>${escapeHtml(testCase.id)}</td>
  <td>${escapeHtml(testCase.status)}</td>
  <td>${(testCase.passRate * 100).toFixed(1)}% / ${(testCase.minPassRate * 100).toFixed(1)}%</td>
  <td>${escapeHtml(testCase.hardGuardStatus)}</td>
  <td>${escapeHtml(testCase.behavior)}</td>
  <td>${escapeHtml(testCase.samples.map((sample) => `#${sample.sampleIndex} ${sample.status}: ${sample.reason}`).join(" | "))}</td>
</tr>`,
  )
  .join("\n");

const caseRows =
  rows ||
  `<tr><td colspan="8">No eval samples are available for this run. ${escapeHtml(runStatus.reason)}</td></tr>`;

function summaryRows(items, idName) {
  const rows = items
    .map(
      (item) => `<tr>
  <td>${escapeHtml(item.provider)}</td>
  <td>${escapeHtml(item.pluginMode)}</td>
  <td>${escapeHtml(item[idName])}</td>
  <td>${item.passed} / ${item.evaluated}${item.blocked > 0 ? ` (${item.blocked} blocked)` : ""}</td>
  <td>${(item.passRate * 100).toFixed(1)}%</td>
  <td>${escapeHtml(item.cases.join(", "))}</td>
</tr>`,
    )
    .join("\n");

  return (
    rows ||
    `<tr><td colspan="6">No ${escapeHtml(idName)} data is available for this run.</td></tr>`
  );
}

const pluginRows = summaryRows(pluginSummaries, "plugin");
const skillRows = summaryRows(skillSummaries, "skill");
const valueGateRows =
  valueGates
    .map(
      (gate) => `<tr>
  <td>${escapeHtml(gate.providerVariant)}</td>
  <td>${escapeHtml(gate.caseId)}</td>
  <td>${escapeHtml(gate.status)}</td>
  <td>${escapeHtml(gate.mode)}</td>
  <td>${gate.fullMarketplacePassRate === null ? "n/a" : `${(gate.fullMarketplacePassRate * 100).toFixed(1)}%`}</td>
  <td>${gate.noPluginsPassRate === null ? "n/a" : `${(gate.noPluginsPassRate * 100).toFixed(1)}%`}</td>
  <td>${gate.lift === null ? "n/a" : `${(gate.lift * 100).toFixed(1)}pp`}</td>
</tr>`,
    )
    .join("\n") ||
  `<tr><td colspan="7">No value-gate data is available for this run.</td></tr>`;

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
    <p><strong>Status:</strong> ${escapeHtml(runStatus.state)}. ${escapeHtml(runStatus.reason)}</p>
    <section class="summary" aria-label="summary">
      <div class="metric"><strong>Total</strong><br>${summary.total}</div>
      <div class="metric"><strong>Passed</strong><br>${summary.passed}</div>
      <div class="metric"><strong>Failed</strong><br>${summary.failed}</div>
      <div class="metric"><strong>Blocked</strong><br>${summary.blocked}</div>
      <div class="metric"><strong>Pass rate</strong><br>${(summary.passRate * 100).toFixed(1)}%</div>
      <div class="metric"><strong>Thresholds met</strong><br>${summary.thresholdsMet}</div>
      <div class="metric"><strong>Thresholds failed</strong><br>${summary.thresholdsFailed}</div>
      <div class="metric"><strong>Thresholds blocked</strong><br>${summary.thresholdsBlocked}</div>
      <div class="metric"><strong>Provider credentials</strong><br>${escapeHtml(runStatus.providerCredentials)}</div>
    </section>
    <table>
      <thead>
        <tr><th>Provider variant</th><th>Plugin mode</th><th>Case</th><th>Status</th><th>Rate / threshold</th><th>Hard guard</th><th>Behavior</th><th>Samples</th></tr>
      </thead>
      <tbody>
${caseRows}
      </tbody>
    </table>
    <h2>Plugin summary</h2>
    <table>
      <thead>
        <tr><th>Provider</th><th>Plugin mode</th><th>Plugin</th><th>Passed</th><th>Pass rate</th><th>Cases</th></tr>
      </thead>
      <tbody>
${pluginRows}
      </tbody>
    </table>
    <h2>Value gates</h2>
    <table>
      <thead>
        <tr><th>Provider variant</th><th>Case</th><th>Status</th><th>Mode</th><th>Full</th><th>No plugins</th><th>Lift</th></tr>
      </thead>
      <tbody>
${valueGateRows}
      </tbody>
    </table>
    <h2>Skill summary</h2>
    <table>
      <thead>
        <tr><th>Provider</th><th>Plugin mode</th><th>Skill</th><th>Passed</th><th>Pass rate</th><th>Cases</th></tr>
      </thead>
      <tbody>
${skillRows}
      </tbody>
    </table>
  </main>
</body>
</html>
`;

fs.writeFileSync(path.join(siteDir, "index.html"), html);
console.log(`wrote ${path.relative(root, siteDir)}/index.html`);
