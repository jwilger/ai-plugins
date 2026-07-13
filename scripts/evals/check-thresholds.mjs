#!/usr/bin/env node
import fs from "node:fs";

const resultsPath = process.argv[2];

if (!resultsPath) {
  console.error("usage: check-thresholds.mjs <results.json>");
  process.exit(2);
}

const raw = JSON.parse(fs.readFileSync(resultsPath, "utf8"));
const results = raw.results?.results || raw.results || [];

if (!Array.isArray(results) || results.length === 0) {
  console.error(`no eval results found: ${resultsPath}`);
  process.exit(1);
}

function resultVars(result) {
  return result.testCase?.vars || result.testCase || result.vars || {};
}

function resultPass(result) {
  const grading = result.gradingResult || {};
  return Boolean(grading.pass ?? result.success ?? result.pass);
}

function resultReason(result) {
  const grading = result.gradingResult || {};
  return String(
    grading.reason ||
      result.reason ||
      grading.error ||
      result.error ||
      result.failureReason ||
      "",
  );
}

function providerId(result) {
  return String(
    result.provider?.label ||
      result.provider?.id ||
      result.provider ||
      result.prompt?.provider ||
      "unknown-provider",
  );
}

function providerVariant(result, vars) {
  return String(
    vars.provider_variant ||
      vars.providerVariant ||
      providerId(result).replace(
        /-(no-plugins|targeted-plugins|full-marketplace)$/,
        "",
      ),
  );
}

function pluginMode(result, vars) {
  return String(
    providerId(result).match(
      /(no-plugins|targeted-plugins|full-marketplace)$/,
    )?.[1] ||
      vars.plugin_mode ||
      vars.pluginMode ||
      "unknown",
  );
}

function isProviderBlocked(reason) {
  return /\b(rate.?limit|weekly limit|session limit|usage limit|insufficient_quota|quota (?:exceeded|exhausted)|(?:exceeded|exhausted) quota|too many requests|429|provider unavailable|could not be resolved)\b/i.test(
    reason,
  );
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function hasErrorEvidence(value) {
  return (
    value !== undefined &&
    value !== null &&
    value !== false &&
    value !== 0 &&
    value !== ""
  );
}

function firstError(entries) {
  const error = entries.find((entry) => hasErrorEvidence(entry));
  return error === undefined ? "" : String(error);
}

function isFinitePositiveNumber(value) {
  return typeof value === "number" && Number.isFinite(value) && value > 0;
}

function isUsableTokenUsage(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }

  const fields = ["prompt", "completion", "total", "cached"];
  if (
    fields.some(
      (field) =>
        typeof value[field] !== "number" ||
        !Number.isFinite(value[field]) ||
        value[field] < 0,
    )
  ) {
    return false;
  }

  return (
    value.total > 0 &&
    value.total === value.prompt + value.completion &&
    value.cached <= value.prompt
  );
}

function hasMeasurementMetadata(vars) {
  return (
    vars.value_gate_mode === "measurement" ||
    vars.valueGateMode === "measurement" ||
    Object.hasOwn(vars, "benchmark_expected_provider_labels") ||
    Object.hasOwn(vars, "benchmarkExpectedProviderLabels") ||
    Object.hasOwn(vars, "benchmark_expected_samples") ||
    Object.hasOwn(vars, "benchmarkExpectedSamples")
  );
}

function gradingError(result) {
  const grading = result.gradingResult;
  if (!grading || typeof grading !== "object") {
    return "";
  }

  const aggregateError = firstError([
    grading.error,
    grading.providerError,
    grading.metadata?.graderError,
    grading.metadata?.error,
    grading.metadata?.providerError,
  ]);
  if (aggregateError) {
    return aggregateError;
  }

  const pendingComponents = Array.isArray(grading.componentResults)
    ? [...grading.componentResults]
    : [];
  const visitedComponents = new Set();
  while (pendingComponents.length > 0) {
    const component = pendingComponents.pop();
    if (
      !component ||
      typeof component !== "object" ||
      visitedComponents.has(component)
    ) {
      continue;
    }
    visitedComponents.add(component);

    const componentError = firstError([
      component.error,
      component.providerError,
      component.response?.error,
      component.response?.providerError,
      component.metadata?.graderError,
      component.metadata?.error,
      component.metadata?.providerError,
    ]);
    if (componentError) {
      return componentError;
    }

    for (const nestedComponent of Array.isArray(component.componentResults)
      ? component.componentResults
      : []) {
      pendingComponents.push(nestedComponent);
    }
  }

  return "";
}

function isValidGradingResult(gradingResult) {
  return (
    gradingResult !== null &&
    typeof gradingResult === "object" &&
    typeof gradingResult.pass === "boolean" &&
    typeof gradingResult.score === "number" &&
    Number.isFinite(gradingResult.score) &&
    typeof gradingResult.reason === "string"
  );
}

const measurementFailures = [];
const measurementCases = new Map();
const operationalErrorPattern =
  /\b(?:codex turn failed|error calling|failed to call|provider (?:error|unavailable)|rate.?limit|quota (?:exceeded|exhausted)|authentication (?:error|failed)|unauthorized|timed? ?out|network error|connection (?:refused|reset))\b/i;
const configuredTests = Array.isArray(raw.config?.tests)
  ? raw.config.tests
  : [];

for (const [testIndex, testCase] of configuredTests.entries()) {
  const vars = testCase?.vars || {};
  if (!hasMeasurementMetadata(vars)) {
    continue;
  }

  const testName = `configured test ${testIndex + 1}`;
  const id = vars.case_id;
  const valueGateMode = vars.value_gate_mode ?? vars.valueGateMode;
  const minPassRate = vars.min_pass_rate ?? vars.minPassRate;
  const expectedLabels =
    vars.benchmark_expected_provider_labels ??
    vars.benchmarkExpectedProviderLabels;
  const expectedSamples =
    vars.benchmark_expected_samples ?? vars.benchmarkExpectedSamples;
  const sampleIndex = vars.sample_index ?? vars.sampleIndex;
  const configuredProviders = testCase.providers;
  const labelsValid =
    Array.isArray(expectedLabels) &&
    expectedLabels.length > 0 &&
    expectedLabels.every(isNonEmptyString) &&
    new Set(expectedLabels).size === expectedLabels.length;
  const normalizedLabels = labelsValid ? [...expectedLabels].sort() : [];
  const providersValid =
    Array.isArray(configuredProviders) &&
    configuredProviders.length > 0 &&
    configuredProviders.every(isNonEmptyString) &&
    new Set(configuredProviders).size === configuredProviders.length &&
    JSON.stringify([...configuredProviders].sort()) ===
      JSON.stringify(normalizedLabels);
  const metadataValid =
    valueGateMode === "measurement" &&
    Number.isFinite(Number(minPassRate)) &&
    Number(minPassRate) === 0 &&
    isNonEmptyString(id) &&
    labelsValid &&
    providersValid &&
    Number.isInteger(expectedSamples) &&
    expectedSamples > 0 &&
    Number.isInteger(sampleIndex) &&
    sampleIndex > 0 &&
    sampleIndex <= expectedSamples;

  if (!metadataValid) {
    measurementFailures.push(
      `${testName}: malformed configured measurement metadata`,
    );
    continue;
  }

  const signature = JSON.stringify({
    expectedLabels: normalizedLabels,
    expectedSamples,
  });
  if (!measurementCases.has(id)) {
    measurementCases.set(id, {
      expectedLabels: normalizedLabels,
      expectedSamples,
      configuredSamples: new Set([sampleIndex]),
      signature,
      rowCounts: new Map(),
    });
  } else if (measurementCases.get(id).signature !== signature) {
    measurementFailures.push(
      `${id}: malformed measurement metadata differs between configured tests`,
    );
  } else {
    measurementCases.get(id).configuredSamples.add(sampleIndex);
  }
}

const hasMeasurementResults = results.some((result) =>
  hasMeasurementMetadata(resultVars(result)),
);
if (hasMeasurementResults && measurementCases.size === 0) {
  measurementFailures.push(
    "measurement results require persisted configured tests",
  );
}

for (const [resultIndex, result] of results.entries()) {
  const vars = resultVars(result);
  if (!hasMeasurementMetadata(vars)) {
    continue;
  }

  const rowName = `result ${resultIndex + 1}`;
  const id = vars.case_id;
  const valueGateMode = vars.value_gate_mode ?? vars.valueGateMode;
  const minPassRate = vars.min_pass_rate ?? vars.minPassRate;
  const expectedLabels =
    vars.benchmark_expected_provider_labels ??
    vars.benchmarkExpectedProviderLabels;
  const expectedSamples =
    vars.benchmark_expected_samples ?? vars.benchmarkExpectedSamples;
  const sampleIndex = vars.sample_index ?? vars.sampleIndex;
  const label = result.provider?.label;

  const labelsValid =
    Array.isArray(expectedLabels) &&
    expectedLabels.length > 0 &&
    expectedLabels.every(isNonEmptyString) &&
    new Set(expectedLabels).size === expectedLabels.length;
  const metadataValid =
    valueGateMode === "measurement" &&
    Number.isFinite(Number(minPassRate)) &&
    Number(minPassRate) === 0 &&
    isNonEmptyString(id) &&
    labelsValid &&
    Number.isInteger(expectedSamples) &&
    expectedSamples > 0 &&
    Number.isInteger(sampleIndex) &&
    sampleIndex > 0 &&
    sampleIndex <= expectedSamples &&
    isNonEmptyString(label);

  if (!metadataValid) {
    measurementFailures.push(`${rowName}: malformed measurement metadata`);
    continue;
  }

  const normalizedLabels = [...expectedLabels].sort();
  const signature = JSON.stringify({
    expectedLabels: normalizedLabels,
    expectedSamples,
  });
  if (!measurementCases.has(id)) {
    measurementFailures.push(
      `${id}: result is absent from configured measurement tests`,
    );
    continue;
  }

  const comparison = measurementCases.get(id);
  if (comparison.signature !== signature) {
    measurementFailures.push(
      `${id}: malformed measurement metadata differs between result rows`,
    );
    continue;
  }
  if (!comparison.configuredSamples.has(sampleIndex)) {
    measurementFailures.push(
      `${id}: sample ${sampleIndex} is absent from configured measurement tests`,
    );
    continue;
  }

  const rowKey = `${label}::${sampleIndex}`;
  comparison.rowCounts.set(rowKey, (comparison.rowCounts.get(rowKey) || 0) + 1);

  if (!expectedLabels.includes(label)) {
    measurementFailures.push(
      `${id}: unexpected result for provider ${label}, sample ${sampleIndex}`,
    );
  }

  const targetError = firstError([
    result.providerError,
    result.response?.error,
    result.response?.providerError,
  ]);
  const targetOutput = result.response?.output;
  const explicitGradingError = gradingError(result);
  const isOperationalError =
    result.failureReason === 2 || result.failureReason === "ERROR";
  const unclassifiedOperationalError =
    !result.gradingResult &&
    isNonEmptyString(result.error) &&
    operationalErrorPattern.test(result.error);

  if (targetError) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: target provider error: ${targetError}`,
    );
  } else if (
    targetOutput === null ||
    targetOutput === undefined ||
    String(targetOutput).trim().length === 0
  ) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: missing target output`,
    );
  }

  if (!isFinitePositiveNumber(result.latencyMs)) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: invalid measurement latency`,
    );
  }
  if (!isUsableTokenUsage(result.tokenUsage)) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: invalid target token usage`,
    );
  }
  if (!isUsableTokenUsage(result.tokenUsage?.assertions)) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: invalid grader token usage`,
    );
  }
  if (
    /\bgpt-5\.6-(?:sol|terra|luna)\b/i.test(label) &&
    !isFinitePositiveNumber(result.cost)
  ) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: invalid measurement cost`,
    );
  }

  if (explicitGradingError) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: grader error: ${explicitGradingError}`,
    );
  } else if (
    !targetError &&
    (isOperationalError || unclassifiedOperationalError)
  ) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: grader error: ${result.error || "unknown grading failure"}`,
    );
  } else if (!targetError && targetOutput != null && !result.gradingResult) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: missing grader result`,
    );
  } else if (
    !targetError &&
    targetOutput != null &&
    !isValidGradingResult(result.gradingResult)
  ) {
    measurementFailures.push(
      `${id}/${label}/sample-${sampleIndex}: malformed grader result`,
    );
  }
}

for (const [id, comparison] of measurementCases) {
  for (const label of comparison.expectedLabels) {
    for (
      let sampleIndex = 1;
      sampleIndex <= comparison.expectedSamples;
      sampleIndex += 1
    ) {
      const rowKey = `${label}::${sampleIndex}`;
      const count = comparison.rowCounts.get(rowKey) || 0;
      if (count === 0) {
        measurementFailures.push(
          `${id}: missing expected result for provider ${label}, sample ${sampleIndex}`,
        );
      } else if (count > 1) {
        measurementFailures.push(
          `${id}: duplicate result for provider ${label}, sample ${sampleIndex}`,
        );
      }
    }
  }
}

const groups = new Map();
const hardGuardFailures = [];

for (const result of results) {
  const vars = resultVars(result);
  const id = String(vars.case_id || result.description || "unknown-case");
  const variant = providerVariant(result, vars);
  const mode = pluginMode(result, vars);
  const key = `${variant}::${mode}::${id}`;
  const reason = resultReason(result);
  const pass = resultPass(result);
  const blocked = !pass && isProviderBlocked(reason);

  const valueGateMode = String(
    vars.value_gate_mode ?? vars.valueGateMode ?? "standard",
  );
  if (
    valueGateMode !== "measurement" &&
    mode !== "no-plugins" &&
    !pass &&
    /\bResponse appears\b/.test(reason)
  ) {
    hardGuardFailures.push(`${key}: ${reason}`);
  }

  if (!groups.has(key)) {
    groups.set(key, {
      key,
      id,
      providerVariant: variant,
      pluginMode: mode,
      minPassRate: Number(vars.min_pass_rate ?? vars.minPassRate ?? 1),
      valueGateMode,
      baselineLiftThreshold: Number(
        vars.baseline_lift_threshold ?? vars.baselineLiftThreshold ?? 0.1,
      ),
      total: 0,
      evaluated: 0,
      passed: 0,
      blocked: 0,
    });
  }

  const group = groups.get(key);
  group.total += 1;
  group.passed += pass ? 1 : 0;
  group.blocked += blocked ? 1 : 0;
  group.evaluated += blocked ? 0 : 1;
  group.minPassRate = Math.max(
    group.minPassRate,
    Number(vars.min_pass_rate ?? vars.minPassRate ?? 1),
  );
}

const failures = [...measurementFailures];
function groupPassRate(group) {
  return group.evaluated === 0 ? 0 : group.passed / group.evaluated;
}

function groupThresholdMet(group) {
  return group.evaluated > 0 && groupPassRate(group) >= group.minPassRate;
}

for (const group of groups.values()) {
  if (group.pluginMode === "no-plugins") {
    continue;
  }
  if (group.valueGateMode === "measurement") {
    continue;
  }
  if (group.evaluated === 0) {
    continue;
  }

  const passRate = group.passed / group.evaluated;
  if (!groupThresholdMet(group)) {
    failures.push(
      `${group.key}: ${group.passed}/${group.evaluated} passed (${(
        passRate * 100
      ).toFixed(
        1,
      )}%) below minPassRate ${(group.minPassRate * 100).toFixed(1)}%`,
    );
  }
}

const groupsByCase = new Map();
for (const group of groups.values()) {
  const key = `${group.providerVariant}::${group.id}`;
  if (!groupsByCase.has(key)) {
    groupsByCase.set(key, []);
  }
  groupsByCase.get(key).push(group);
}

for (const [key, caseGroups] of groupsByCase) {
  const full = caseGroups.find(
    (group) => group.pluginMode === "full-marketplace",
  );
  const targeted = caseGroups.find(
    (group) => group.pluginMode === "targeted-plugins",
  );
  const baseline = caseGroups.find(
    (group) => group.pluginMode === "no-plugins",
  );

  if (!baseline || (!full && !targeted)) {
    continue;
  }

  const reference = full || targeted;
  const plugin = full || targeted;

  if (reference.valueGateMode === "none") {
    continue;
  }
  if (reference.valueGateMode === "measurement") {
    continue;
  }

  const pluginComplete = plugin.evaluated > 0 && plugin.blocked === 0;
  const baselineComplete = baseline.evaluated > 0 && baseline.blocked === 0;
  const pluginPass = groupThresholdMet(plugin);
  const baselinePass = groupThresholdMet(baseline);
  const lift = groupPassRate(plugin) - groupPassRate(baseline);
  const valueGatePass =
    reference.valueGateMode === "safety-critical"
      ? pluginComplete && baselineComplete && pluginPass && !baselinePass
      : pluginComplete &&
        baselineComplete &&
        pluginPass &&
        lift >= reference.baselineLiftThreshold;

  if (!valueGatePass) {
    const reason =
      !pluginComplete || !baselineComplete
        ? "missing complete plugin or baseline evidence"
        : reference.valueGateMode === "safety-critical"
          ? "safety-critical value gate requires plugin pass and no-plugin baseline miss"
          : `standard value gate requires lift >= ${reference.baselineLiftThreshold}`;
    failures.push(
      `${key}: ${reason} (plugin ${(groupPassRate(plugin) * 100).toFixed(
        1,
      )}%, no-plugins ${(groupPassRate(baseline) * 100).toFixed(1)}%)`,
    );
  }
}

if (hardGuardFailures.length > 0) {
  console.error("Hard guard failures:");
  for (const failure of hardGuardFailures) {
    console.error(`- ${failure}`);
  }
}

if (failures.length > 0) {
  console.error("Eval thresholds failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
}

if (hardGuardFailures.length > 0 || failures.length > 0) {
  process.exit(1);
}

console.error(
  `Eval thresholds passed for ${groups.size} aggregate(s) using plugin-mode minPassRate and value gates.`,
);
