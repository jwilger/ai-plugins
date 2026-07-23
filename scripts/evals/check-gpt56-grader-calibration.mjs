#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { isDeepStrictEqual } from "node:util";
import { fileURLToPath } from "node:url";

const resultsPath = process.argv[2];

if (!resultsPath) {
  console.error("usage: check-gpt56-grader-calibration.mjs <results.json>");
  process.exit(2);
}

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const benchmarkDir = path.join(root, "evals/benchmarks/gpt-5.6-model-family");
const require = createRequire(import.meta.url);
const { parse: parseYaml } = require("yaml");
const loadCases = require(path.join(benchmarkDir, "grader-cases.cjs"));
const expectedCases = loadCases();
const calibrationConfig = parseYaml(
  fs.readFileSync(
    path.join(benchmarkDir, "grader-promptfooconfig.yaml"),
    "utf8",
  ),
);

function isNonemptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function isPassingScore(value) {
  return typeof value === "number" && Number.isFinite(value) && value > 0;
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

function firstErrorField(entries) {
  return entries.find(([, value]) => hasErrorEvidence(value))?.[0];
}

function exactKeys(value, expectedKeys) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const actualKeys = Object.keys(value).sort();
  return (
    JSON.stringify(actualKeys) === JSON.stringify([...expectedKeys].sort())
  );
}

function reportFailures(failures) {
  console.error("GPT-5.6 grader calibration verification failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

function providerLabel(provider) {
  if (typeof provider === "string") {
    return provider;
  }
  return (
    provider?.label ||
    provider?.options?.label ||
    provider?.id ||
    provider?.providerId
  );
}

function providerId(provider) {
  return (
    provider?.providerId ||
    provider?.options?.id ||
    (typeof provider?.id === "string" ? provider.id : undefined)
  );
}

function providerConfig(provider) {
  return provider?.config || provider?.options?.config;
}

function assertionSemanticIdentity(assertion) {
  return JSON.stringify([
    providerLabel(assertion?.provider),
    assertion?.metric,
    assertion?.type,
    assertion?.value,
  ]);
}

function assertionHasExactFields(assertion) {
  return exactKeys(assertion, ["type", "value", "provider", "metric"]);
}

function componentLabel(component) {
  const assertion = component?.assertion;
  return `${providerLabel(assertion?.provider) ?? "unknown-provider"} / ${assertion?.metric ?? "unknown-metric"}`;
}

function resolvedConfiguredProvider(provider) {
  const config = structuredClone(provider.config);
  config.working_dir =
    process.env.GPT56_BENCHMARK_WORKSPACE ||
    path.join(root, ".evals/agent-workspace");
  config.cli_env.CODEX_HOME =
    process.env.CODEX_EVAL_HOME_NO_PLUGINS ||
    path.join(root, ".evals/codex-home-no-plugins");
  return {
    id: provider.id,
    label: provider.label,
    config,
  };
}

function providerMatchesConfiguration(provider, expectedProvider) {
  if (
    providerLabel(provider) !== expectedProvider.label ||
    providerId(provider) !== expectedProvider.id
  ) {
    return false;
  }

  const actualConfig = providerConfig(provider);
  if (!actualConfig || typeof actualConfig !== "object") {
    return false;
  }
  const { basePath, ...behaviorConfig } = actualConfig;
  if (basePath !== undefined && path.resolve(basePath) !== benchmarkDir) {
    return false;
  }
  return isDeepStrictEqual(behaviorConfig, expectedProvider.config);
}

const configurationFailures = [];
if (!Array.isArray(expectedCases) || expectedCases.length !== 8) {
  configurationFailures.push(
    `expected exactly 8 configured GPT-5.6 calibration cases, got ${Array.isArray(expectedCases) ? expectedCases.length : "a non-array"}`,
  );
}

const configuredGraders = new Map();
for (const provider of calibrationConfig.providers || []) {
  if (provider.id !== "file://trace-enforced-codex-provider.mjs") {
    continue;
  }
  if (
    !isNonemptyString(provider.label) ||
    configuredGraders.has(provider.label)
  ) {
    configurationFailures.push(
      `invalid or duplicate configured calibration grader ${provider.label ?? "<missing label>"}`,
    );
    continue;
  }
  configuredGraders.set(provider.label, resolvedConfiguredProvider(provider));
}
if (configuredGraders.size !== 3) {
  configurationFailures.push(
    `expected exactly 3 configured calibration graders, got ${configuredGraders.size}`,
  );
}

const expectedByDescription = new Map();
for (const expectedCase of Array.isArray(expectedCases) ? expectedCases : []) {
  if (!isNonemptyString(expectedCase.description)) {
    configurationFailures.push(
      "configured calibration case has no description",
    );
    continue;
  }
  if (expectedByDescription.has(expectedCase.description)) {
    configurationFailures.push(
      `duplicate configured calibration case ${expectedCase.description}`,
    );
  }
  expectedByDescription.set(expectedCase.description, expectedCase);

  const assertions = expectedCase.assert;
  const identities = Array.isArray(assertions)
    ? assertions.map(assertionSemanticIdentity)
    : [];
  if (identities.length !== 3 || new Set(identities).size !== 3) {
    configurationFailures.push(
      `${expectedCase.description}: configuration must define exactly 3 unique grader assertions`,
    );
  }
  for (const assertion of Array.isArray(assertions) ? assertions : []) {
    if (!configuredGraders.has(assertion.provider)) {
      configurationFailures.push(
        `${expectedCase.description}: assertion references unknown grader ${assertion.provider}`,
      );
    }
  }
}

if (configurationFailures.length > 0) {
  reportFailures(configurationFailures);
}

let artifact;
try {
  artifact = JSON.parse(fs.readFileSync(resultsPath, "utf8"));
} catch (error) {
  reportFailures([`cannot read calibration artifact: ${error.message}`]);
}

const results = artifact.results?.results || artifact.results;
if (!Array.isArray(results)) {
  reportFailures(["calibration artifact has no results array"]);
}

const failures = [];
if (results.length !== expectedCases.length) {
  failures.push(
    `expected exactly ${expectedCases.length} calibration results, got ${results.length}`,
  );
}

const rowsByDescription = new Map();
for (const result of results) {
  const description = result.testCase?.description;
  if (!isNonemptyString(description)) {
    failures.push("unknown calibration case <missing description>");
    continue;
  }
  if (!expectedByDescription.has(description)) {
    failures.push(`unknown calibration case ${description}`);
    continue;
  }
  const rows = rowsByDescription.get(description) || [];
  rows.push(result);
  rowsByDescription.set(description, rows);
}

for (const expectedCase of expectedCases) {
  const rows = rowsByDescription.get(expectedCase.description) || [];
  if (rows.length === 0) {
    failures.push(
      `missing configured calibration case ${expectedCase.description}`,
    );
    continue;
  }
  if (rows.length > 1) {
    failures.push(`duplicate calibration case ${expectedCase.description}`);
    continue;
  }

  const result = rows[0];
  const label = expectedCase.description;
  const expectedAssertions = expectedCase.assert;
  const expectedAssertionByIdentity = new Map(
    expectedAssertions.map((assertion) => [
      assertionSemanticIdentity(assertion),
      assertion,
    ]),
  );
  const expectedMetrics = expectedAssertions.map(
    (assertion) => assertion.metric,
  );

  if (result.success !== true) {
    failures.push(`${label}: target success evidence is missing`);
  }
  if (
    result.provider?.id !== "echo" ||
    result.provider?.label !== "frozen-human-answer"
  ) {
    failures.push(`${label}: target provider must be frozen-human-answer/echo`);
  }
  const targetErrorField = firstErrorField([
    ["result.error", result.error],
    ["result.providerError", result.providerError],
    ["result.failureReason", result.failureReason],
    ["response.error", result.response?.error],
    ["response.providerError", result.response?.providerError],
  ]);
  if (targetErrorField) {
    failures.push(`${label}: target/provider error at ${targetErrorField}`);
  }
  if (result.response?.output !== expectedCase.vars.candidate_output) {
    failures.push(`${label}: target output does not match the frozen answer`);
  }
  if (!isDeepStrictEqual(result.vars, expectedCase.vars)) {
    failures.push(`${label}: top-level result vars do not match configuration`);
  }
  if (!isDeepStrictEqual(result.testCase?.vars, expectedCase.vars)) {
    failures.push(
      `${label}: persisted case vars do not exactly match configuration`,
    );
  }

  if (!isPassingScore(result.score)) {
    failures.push(`${label}: result score is not valid passing evidence`);
  }
  if (!exactKeys(result.namedScores, expectedMetrics)) {
    failures.push(
      `${label}: result named-score keys do not match configured graders`,
    );
  } else if (
    expectedMetrics.some(
      (metric) => !isPassingScore(result.namedScores[metric]),
    )
  ) {
    failures.push(
      `${label}: result named scores are not valid passing evidence`,
    );
  }

  const artifactAssertions = result.testCase?.assert;
  if (!Array.isArray(artifactAssertions) || artifactAssertions.length !== 3) {
    failures.push(
      `${label}: expected exactly 3 persisted grader assertions, got ${Array.isArray(artifactAssertions) ? artifactAssertions.length : 0}`,
    );
  } else {
    const actualIdentities = [];
    for (const assertion of artifactAssertions) {
      if (!assertionHasExactFields(assertion)) {
        failures.push(
          `${label}: persisted grader assertion has unexpected fields`,
        );
      }
      const identity = assertionSemanticIdentity(assertion);
      actualIdentities.push(identity);
      const expectedAssertion = expectedAssertionByIdentity.get(identity);
      if (!expectedAssertion) {
        continue;
      }
      if (
        !providerMatchesConfiguration(
          assertion.provider,
          configuredGraders.get(expectedAssertion.provider),
        )
      ) {
        failures.push(
          `${label}: persisted grader assertion provider does not match configuration`,
        );
      }
    }
    if (
      new Set(actualIdentities).size !== 3 ||
      actualIdentities.some(
        (identity) => !expectedAssertionByIdentity.has(identity),
      )
    ) {
      failures.push(
        `${label}: persisted grader assertions do not match configuration`,
      );
    }
  }

  const grading = result.gradingResult;
  if (!grading || typeof grading !== "object") {
    failures.push(`${label}: grading result is missing`);
    continue;
  }
  const gradingErrorField = firstErrorField([
    ["gradingResult.error", grading.error],
    ["gradingResult.providerError", grading.providerError],
    ["gradingResult.failureReason", grading.failureReason],
    ["gradingResult.metadata.graderError", grading.metadata?.graderError],
  ]);
  if (gradingErrorField) {
    failures.push(`${label}: grading error at ${gradingErrorField}`);
  }
  if (
    grading.pass !== true ||
    !isPassingScore(grading.score) ||
    !isNonemptyString(grading.reason)
  ) {
    failures.push(`${label}: grading result is not valid passing evidence`);
  }
  if (!exactKeys(grading.namedScores, expectedMetrics)) {
    failures.push(
      `${label}: grading named-score keys do not match configured graders`,
    );
  } else if (
    expectedMetrics.some(
      (metric) => !isPassingScore(grading.namedScores[metric]),
    )
  ) {
    failures.push(
      `${label}: grading named scores are not valid passing evidence`,
    );
  }

  const components = grading.componentResults;
  if (!Array.isArray(components) || components.length !== 3) {
    failures.push(
      `${label}: expected exactly 3 grader components, got ${Array.isArray(components) ? components.length : 0}`,
    );
    continue;
  }

  const componentsByIdentity = new Map();
  for (const component of components) {
    if (!assertionHasExactFields(component.assertion)) {
      failures.push(
        `${label}: grader component assertion has unexpected fields`,
      );
    }
    const identity = assertionSemanticIdentity(component.assertion);
    const expectedAssertion = expectedAssertionByIdentity.get(identity);
    if (!expectedAssertion) {
      failures.push(
        `${label}: unexpected grader component ${componentLabel(component)}`,
      );
      continue;
    }
    if (
      !providerMatchesConfiguration(
        component.assertion.provider,
        configuredGraders.get(expectedAssertion.provider),
      )
    ) {
      failures.push(`${label}: unexpected grader component provider identity`);
    }
    const matching = componentsByIdentity.get(identity) || [];
    matching.push(component);
    componentsByIdentity.set(identity, matching);
  }

  let scoreMismatch = result.score !== grading.score;
  for (const [identity, expectedAssertion] of expectedAssertionByIdentity) {
    const matching = componentsByIdentity.get(identity) || [];
    if (matching.length !== 1) {
      failures.push(
        `${label}: expected exactly one grader component ${expectedAssertion.provider} / ${expectedAssertion.metric}, got ${matching.length}`,
      );
      continue;
    }
    const component = matching[0];
    const componentErrorField = firstErrorField([
      ["component.error", component.error],
      ["component.providerError", component.providerError],
      ["component.failureReason", component.failureReason],
      ["component.response.error", component.response?.error],
      ["component.metadata.error", component.metadata?.error],
      ["component.metadata.graderError", component.metadata?.graderError],
    ]);
    if (componentErrorField) {
      failures.push(
        `${label}: grader component ${expectedAssertion.provider} has an error at ${componentErrorField}`,
      );
    }
    if (
      component.pass !== true ||
      !isPassingScore(component.score) ||
      !isNonemptyString(component.reason)
    ) {
      failures.push(
        `${label}: grader component ${expectedAssertion.provider} is not valid passing evidence`,
      );
    }
    if (
      !exactKeys(result.namedScores, expectedMetrics) ||
      !exactKeys(grading.namedScores, expectedMetrics) ||
      component.score !== result.namedScores[expectedAssertion.metric] ||
      component.score !== grading.namedScores[expectedAssertion.metric]
    ) {
      scoreMismatch = true;
    }
  }
  if (scoreMismatch) {
    failures.push(
      `${label}: named scores do not match grader component scores`,
    );
  }
}

if (failures.length > 0) {
  reportFailures(failures);
}

console.error(
  `verified ${results.length} complete GPT-5.6 grader calibration results across 3 configured graders`,
);
