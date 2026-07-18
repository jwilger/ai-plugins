#!/usr/bin/env node
import fs from "node:fs";
import { pathToFileURL } from "node:url";

const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const expectedConditions = [
  "no-marketplace-skills",
  "targeted-quality-skills",
  "all-marketplace-skills",
];
const expectedTargetedPlugins = [
  "advisor",
  "development-discipline",
  "engineering-standards",
];
const expectedPerRunMetrics = [
  "conjunctive-success",
  "outcome-class",
  "latency",
  "tokens",
  "cost",
];
const expectedAggregateMetrics = [
  "success-rate",
  "pass@3-capability",
  "pass^3-reliability",
];
const expectedRustFeatureGates = [
  "source-rebuild",
  "black-box-behavior",
  "regression-tests",
  "baseline-regression-replay",
  "format",
  "clippy",
  "diff-scope",
  "safety",
];

function assertObject(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
}

function assertIdentifier(value, label) {
  if (typeof value !== "string" || !identifierPattern.test(value)) {
    throw new Error(`invalid ${label}: ${String(value)}`);
  }
}

function assertCanonicalPluginList(value, label) {
  if (!Array.isArray(value)) {
    throw new Error(`${label} plugins must be an array`);
  }
  for (const plugin of value) {
    assertIdentifier(plugin, `${label} plugin`);
  }
  const canonical = [...new Set(value)].sort();
  if (JSON.stringify(canonical) !== JSON.stringify(value)) {
    throw new Error(`${label} plugins must be unique and sorted`);
  }
}

function assertExactArray(value, expected, label) {
  if (
    !Array.isArray(value) ||
    JSON.stringify(value) !== JSON.stringify(expected)
  ) {
    throw new Error(`${label} must be exactly: ${expected.join(", ")}`);
  }
}

export function validateBenchmarkContract(contract) {
  assertObject(contract, "benchmark contract");
  if (contract.schemaVersion !== 1) {
    throw new Error("benchmark contract schemaVersion must be 1");
  }
  assertIdentifier(contract.id, "benchmark id");
  if (contract.sampleCount !== 3) {
    throw new Error("benchmark sampleCount must be exactly 3");
  }

  if (!Array.isArray(contract.conditions)) {
    throw new Error("benchmark conditions must be an array");
  }
  const conditionIds = contract.conditions.map((condition) => {
    assertObject(condition, "benchmark condition");
    assertIdentifier(condition.id, "condition id");
    return condition.id;
  });
  if (JSON.stringify(conditionIds) !== JSON.stringify(expectedConditions)) {
    throw new Error(
      `benchmark conditions must be exactly: ${expectedConditions.join(", ")}`,
    );
  }
  if (contract.promotionEligible !== false) {
    throw new Error("benchmark must remain non-promotional");
  }
  if (
    typeof contract.claim !== "string" ||
    !contract.claim.startsWith("Non-promotional directional evidence")
  ) {
    throw new Error("benchmark claim must remain explicitly non-promotional");
  }
  assertObject(contract.provider, "benchmark provider");
  const expectedProvider = {
    id: "openai:codex-sdk",
    model: "gpt-5.6-terra",
    reasoningEffort: "medium",
    sandboxMode: "workspace-write",
    approvalPolicy: "never",
    networkAccess: false,
    authentication: "chatgpt-login-disposable-copy",
  };
  for (const [key, expected] of Object.entries(expectedProvider)) {
    if (contract.provider[key] !== expected) {
      const label =
        key === "authentication"
          ? "provider authentication"
          : `provider ${key}`;
      throw new Error(`${label} must be ${String(expected)}`);
    }
  }
  assertCanonicalPluginList(contract.conditions[0].plugins, "no-marketplace-skills");
  if (contract.conditions[0].plugins.length !== 0) {
    throw new Error("no-marketplace-skills composition must be empty");
  }
  if (contract.conditions[0].surface !== "codex-bundled-skills-only") {
    throw new Error(
      "no-marketplace-skills surface must be codex-bundled-skills-only",
    );
  }
  assertCanonicalPluginList(
    contract.conditions[1].plugins,
    "targeted-quality-skills",
  );
  assertExactArray(
    contract.conditions[1].plugins,
    expectedTargetedPlugins,
    "targeted-quality-skills plugins",
  );
  if (contract.conditions[1].surface !== "skills-only") {
    throw new Error("targeted-quality-skills surface must be skills-only");
  }
  if (
    contract.conditions[2].plugins !== "codex-marketplace-skills-at-run-start"
  ) {
    throw new Error(
      "all-marketplace-skills composition must resolve from the Codex marketplace at run start",
    );
  }
  if (contract.conditions[2].surface !== "skills-only") {
    throw new Error("all-marketplace-skills surface must be skills-only");
  }

  if (!Array.isArray(contract.cases) || contract.cases.length === 0) {
    throw new Error("benchmark cases must be a non-empty array");
  }
  const caseIds = new Set();
  for (const testCase of contract.cases) {
    assertObject(testCase, "benchmark case");
    assertIdentifier(testCase.id, "case id");
    assertIdentifier(testCase.fixture, "fixture id");
    if (caseIds.has(testCase.id)) {
      throw new Error(`duplicate case id: ${testCase.id}`);
    }
    caseIds.add(testCase.id);
    if (!["feature", "bugfix", "refactor"].includes(testCase.taskType)) {
      throw new Error(`invalid task type for ${testCase.id}`);
    }
    if (
      !Array.isArray(testCase.deterministicGates) ||
      testCase.deterministicGates.length === 0 ||
      testCase.deterministicGates.some(
        (gate) => typeof gate !== "string" || !identifierPattern.test(gate),
      )
    ) {
      throw new Error(`invalid deterministic gates for ${testCase.id}`);
    }
  }
  if (
    contract.cases.length !== 1 ||
    contract.cases[0].id !== "rust-cli-feature"
  ) {
    throw new Error("benchmark must contain exactly the rust-cli-feature case");
  }
  const rustFeature = contract.cases[0];
  if (rustFeature.taskType !== "feature") {
    throw new Error("rust-cli-feature taskType must be feature");
  }
  if (rustFeature.fixture !== "expense-report") {
    throw new Error("rust-cli-feature fixture must be expense-report");
  }
  assertExactArray(
    rustFeature.deterministicGates,
    expectedRustFeatureGates,
    "rust-cli-feature deterministic gates",
  );

  assertObject(contract.metrics, "benchmark metrics");
  assertExactArray(
    contract.metrics.perRun,
    expectedPerRunMetrics,
    "benchmark per-run metrics",
  );
  assertExactArray(
    contract.metrics.aggregates,
    expectedAggregateMetrics,
    "benchmark aggregate metrics",
  );

  assertObject(contract.diagnosticGates, "benchmark diagnostic gates");
  const expectedTurns =
    contract.cases.length * contract.conditions.length * contract.sampleCount;
  if (contract.diagnosticGates.expectedExecutionTurns !== expectedTurns) {
    throw new Error(
      "expectedExecutionTurns must equal cases x conditions x samples",
    );
  }
  if (
    contract.diagnosticGates.completeRuns !==
    contract.diagnosticGates.expectedExecutionTurns
  ) {
    throw new Error("completeRuns must equal expectedExecutionTurns");
  }
  for (const key of [
    "providerErrors",
    "operationalErrors",
    "provenanceErrors",
    "safetyFailures",
  ]) {
    if (contract.diagnosticGates[key] !== 0) {
      throw new Error(`${key} must be zero`);
    }
  }
  if (
    contract.diagnosticGates.candidateFailuresAreMeasurementOutcomes !== true
  ) {
    throw new Error("candidate failures must remain measurement outcomes");
  }

  return contract;
}

export function loadBenchmarkContract(contractFile) {
  const parsed = JSON.parse(fs.readFileSync(contractFile, "utf8"));
  return validateBenchmarkContract(parsed);
}

const invokedFile = process.argv[1]
  ? pathToFileURL(process.argv[1]).href
  : undefined;
if (import.meta.url === invokedFile) {
  try {
    const contractFile = process.argv[2];
    if (!contractFile) {
      throw new Error(
        "usage: validate-code-quality-contract.mjs <benchmark.json>",
      );
    }
    loadBenchmarkContract(contractFile);
    console.log("code-quality benchmark contract is valid");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(2);
  }
}
