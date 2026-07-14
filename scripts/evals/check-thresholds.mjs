#!/usr/bin/env node
import fs from "node:fs";

const resultsPath = process.argv[2];

if (!resultsPath) {
  console.error("usage: check-thresholds.mjs <results.json>");
  process.exit(2);
}

let raw;
try {
  raw = JSON.parse(fs.readFileSync(resultsPath, "utf8"));
} catch {
  console.error("invalid eval artifact: results JSON could not be read");
  process.exit(1);
}
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

const failures = [];
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
