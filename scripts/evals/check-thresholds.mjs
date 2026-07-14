#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const resultsPath = process.argv[2];
const remainingArgs = process.argv.slice(3);
let expectedMeasurementConfigPath;

for (let index = 0; index < remainingArgs.length; index += 1) {
  const argument = remainingArgs[index];
  if (argument !== "--expected-measurement-config") {
    console.error("unknown argument");
    process.exit(2);
  }

  const value = remainingArgs[index + 1];
  if (!value || value.startsWith("--")) {
    console.error("--expected-measurement-config requires a YAML path");
    process.exit(2);
  }
  if (expectedMeasurementConfigPath) {
    console.error("--expected-measurement-config may be specified only once");
    process.exit(2);
  }

  expectedMeasurementConfigPath = path.resolve(value);
  index += 1;
}

if (!resultsPath) {
  console.error(
    "usage: check-thresholds.mjs <results.json> [--expected-measurement-config <promptfooconfig.yaml>]",
  );
  process.exit(2);
}

class SourceContractError extends Error {
  constructor(contractPath, reason) {
    super(`${contractPath}: ${reason}`);
  }
}

function isPlainObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function hasExactKeys(value, keys) {
  return (
    isPlainObject(value) &&
    JSON.stringify(Object.keys(value).sort()) ===
      JSON.stringify([...keys].sort())
  );
}

function requireExactKeys(value, keys, contractPath) {
  if (!hasExactKeys(value, keys)) {
    throw new SourceContractError(
      contractPath,
      "has unexpected or missing keys",
    );
  }
}

function requireNonemptyString(value, contractPath) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new SourceContractError(contractPath, "must be a nonempty string");
  }
}

function isWithinDirectory(directory, candidate) {
  const relative = path.relative(directory, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

const exactEnvTemplate = /^\{\{\s*env\.([A-Za-z_][A-Za-z0-9_]*)\s*\}\}$/;
const anyEnvTemplate = /\{\{[^{}]*\benv\./;

function resolveSourceEnvironment(value, contractPath = "source") {
  if (Array.isArray(value)) {
    return value.map((entry, index) =>
      resolveSourceEnvironment(entry, `${contractPath}[${index}]`),
    );
  }
  if (isPlainObject(value)) {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [
        key,
        resolveSourceEnvironment(entry, `${contractPath}.${key}`),
      ]),
    );
  }
  if (typeof value !== "string") {
    return value;
  }

  const match = value.match(exactEnvTemplate);
  if (match) {
    const resolved = process.env[match[1]];
    if (typeof resolved !== "string" || resolved.length === 0) {
      throw new SourceContractError(
        contractPath,
        "required environment reference is unresolved",
      );
    }
    return resolved;
  }
  if (anyEnvTemplate.test(value)) {
    throw new SourceContractError(
      contractPath,
      "environment references must occupy the complete string",
    );
  }
  return value;
}

function validateProviderDescriptor(provider, contractPath) {
  requireExactKeys(provider, ["id", "label", "config"], contractPath);
  requireNonemptyString(provider.id, `${contractPath}.id`);
  requireNonemptyString(provider.label, `${contractPath}.label`);
  if (!provider.id.startsWith("file://")) {
    throw new SourceContractError(
      `${contractPath}.id`,
      "must name a local provider",
    );
  }
  if (
    !isPlainObject(provider.config) ||
    Object.keys(provider.config).length === 0
  ) {
    throw new SourceContractError(
      `${contractPath}.config`,
      "must be a nonempty object",
    );
  }
}

function validateExpectedMeasurementSource(source) {
  requireExactKeys(
    source,
    [
      "description",
      "prompts",
      "providers",
      "tests",
      "defaultTest",
      "commandLineOptions",
      "metadata",
    ],
    "source",
  );
  requireNonemptyString(source.description, "source.description");

  if (
    !Array.isArray(source.prompts) ||
    source.prompts.length !== 1 ||
    typeof source.prompts[0] !== "string" ||
    source.prompts[0].length === 0
  ) {
    throw new SourceContractError(
      "source.prompts",
      "must contain exactly one prompt template",
    );
  }
  const promptExpressions = source.prompts[0].match(/\{\{[^{}]+\}\}/g) ?? [];
  if (
    promptExpressions.length !== 1 ||
    !/^\{\{\s*scenario_prompt\s*\}\}$/.test(promptExpressions[0])
  ) {
    throw new SourceContractError(
      "source.prompts[0]",
      "must contain exactly one scenario_prompt expression",
    );
  }

  if (!Array.isArray(source.providers) || source.providers.length === 0) {
    throw new SourceContractError(
      "source.providers",
      "must be a nonempty array",
    );
  }
  const providerLabels = new Set();
  for (const [index, provider] of source.providers.entries()) {
    const providerPath = `source.providers[${index}]`;
    validateProviderDescriptor(provider, providerPath);
    if (providerLabels.has(provider.label)) {
      throw new SourceContractError(`${providerPath}.label`, "must be unique");
    }
    providerLabels.add(provider.label);
  }

  requireExactKeys(source.defaultTest, ["options"], "source.defaultTest");
  requireExactKeys(
    source.defaultTest.options,
    ["provider"],
    "source.defaultTest.options",
  );
  requireExactKeys(
    source.defaultTest.options.provider,
    ["text"],
    "source.defaultTest.options.provider",
  );
  validateProviderDescriptor(
    source.defaultTest.options.provider.text,
    "source.defaultTest.options.provider.text",
  );

  requireExactKeys(
    source.commandLineOptions,
    ["maxConcurrency", "share", "cache", "write"],
    "source.commandLineOptions",
  );
  if (
    !Number.isInteger(source.commandLineOptions.maxConcurrency) ||
    source.commandLineOptions.maxConcurrency < 1 ||
    source.commandLineOptions.maxConcurrency > 2
  ) {
    throw new SourceContractError(
      "source.commandLineOptions.maxConcurrency",
      "must be an integer from 1 through 2",
    );
  }
  if (
    source.commandLineOptions.cache !== false ||
    source.commandLineOptions.share !== false ||
    source.commandLineOptions.write !== true
  ) {
    throw new SourceContractError(
      "source.commandLineOptions",
      "must keep cache and sharing disabled and writes enabled",
    );
  }
  if (!isPlainObject(source.metadata)) {
    throw new SourceContractError("source.metadata", "must be an object");
  }
}

function loadExpectedMeasurementTests(modulePath) {
  const require = createRequire(import.meta.url);
  const loaded = require(modulePath);
  const loadTests = loaded?.default ?? loaded;
  if (typeof loadTests !== "function") {
    throw new TypeError("source.tests must export a function");
  }

  const tests = loadTests();
  if (!Array.isArray(tests) || tests.length === 0) {
    throw new TypeError("source.tests must return a nonempty array");
  }
  return tests;
}

function loadExpectedMeasurementContract(configPath) {
  let parseYaml;
  try {
    ({ parse: parseYaml } = createRequire(import.meta.url)("yaml"));
  } catch {
    throw new SourceContractError(
      "source",
      "YAML parser dependency is unavailable",
    );
  }

  let realConfigPath;
  let sourceText;
  try {
    realConfigPath = fs.realpathSync(configPath);
    if (!fs.statSync(realConfigPath).isFile()) {
      throw new SourceContractError("source", "YAML path is not a file");
    }
    sourceText = fs.readFileSync(realConfigPath, "utf8");
  } catch (error) {
    if (error instanceof SourceContractError) {
      throw error;
    }
    throw new SourceContractError("source", "YAML could not be read");
  }

  let parsed;
  try {
    parsed = parseYaml(sourceText);
  } catch {
    throw new SourceContractError("source", "invalid YAML");
  }
  if (!isPlainObject(parsed)) {
    throw new SourceContractError("source", "YAML root must be an object");
  }

  const source = resolveSourceEnvironment(parsed);
  validateExpectedMeasurementSource(source);
  if (typeof source.tests !== "string" || !source.tests.startsWith("file://")) {
    throw new SourceContractError(
      "source.tests",
      "must be a relative local file URL",
    );
  }
  const relativeTestsPath = source.tests.slice("file://".length);
  if (
    relativeTestsPath.length === 0 ||
    path.isAbsolute(relativeTestsPath) ||
    path.extname(relativeTestsPath) !== ".cjs"
  ) {
    throw new SourceContractError(
      "source.tests",
      "must name a relative CommonJS module",
    );
  }

  const benchmarkDirectory = fs.realpathSync(path.dirname(realConfigPath));
  const unresolvedTestsPath = path.resolve(
    benchmarkDirectory,
    relativeTestsPath,
  );
  let testsPath;
  try {
    testsPath = fs.realpathSync(unresolvedTestsPath);
    if (
      !isWithinDirectory(benchmarkDirectory, testsPath) ||
      !fs.statSync(testsPath).isFile()
    ) {
      throw new SourceContractError(
        "source.tests",
        "must stay within the benchmark directory",
      );
    }
  } catch (error) {
    if (error instanceof SourceContractError) {
      throw error;
    }
    throw new SourceContractError("source.tests", "module could not be read");
  }

  let tests;
  try {
    tests = loadExpectedMeasurementTests(testsPath);
  } catch (error) {
    if (error instanceof SourceContractError) {
      throw error;
    }
    throw new SourceContractError("source.tests", "module could not be loaded");
  }

  let expectedRuntimeMaxConcurrency = source.commandLineOptions.maxConcurrency;
  if (process.env.PROMPTFOO_MAX_CONCURRENCY !== undefined) {
    if (!/^[12]$/.test(process.env.PROMPTFOO_MAX_CONCURRENCY)) {
      throw new SourceContractError(
        "runtime.maxConcurrency",
        "override must be a canonical integer from 1 through 2",
      );
    }
    expectedRuntimeMaxConcurrency = Number(
      process.env.PROMPTFOO_MAX_CONCURRENCY,
    );
  }

  return {
    benchmarkDirectory,
    expectedRuntimeMaxConcurrency,
    source,
    tests,
  };
}

let expectedMeasurementContract;
if (expectedMeasurementConfigPath) {
  try {
    expectedMeasurementContract = loadExpectedMeasurementContract(
      expectedMeasurementConfigPath,
    );
  } catch (error) {
    console.error(
      `invalid expected measurement config: ${
        error instanceof Error ? error.message : "source: invalid contract"
      }`,
    );
    process.exit(2);
  }
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
const expectedMeasurementTests = expectedMeasurementContract?.tests;

function measurementTestKey(testCase) {
  const vars = testCase?.vars || {};
  const id = vars.case_id;
  const sampleIndex = vars.sample_index ?? vars.sampleIndex;
  if (!isNonEmptyString(id) || !Number.isInteger(sampleIndex)) {
    return undefined;
  }
  return `${id}::${sampleIndex}`;
}

function stableJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .filter((key) => value[key] !== undefined)
      .map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function addContractDifference(contractPath) {
  measurementFailures.push(
    `${contractPath}: differs from expected measurement config`,
  );
}

function compareContractValue(contractPath, actual, expected) {
  if (stableJson(actual) !== stableJson(expected)) {
    addContractDifference(contractPath);
  }
}

function validatePersistedMeasurementConfig(contract) {
  const artifactConfig = raw.config;
  const expectedConfigKeys = [
    "tags",
    "description",
    "prompts",
    "providers",
    "tests",
    "env",
    "defaultTest",
    "outputPath",
    "extensions",
    "metadata",
    "evaluateOptions",
  ];
  if (!hasExactKeys(artifactConfig, expectedConfigKeys)) {
    addContractDifference("artifact.config keys");
  }

  const source = contract.source;
  compareContractValue(
    "artifact.config.description",
    artifactConfig?.description,
    source.description,
  );
  compareContractValue(
    "artifact.config.prompts",
    artifactConfig?.prompts,
    source.prompts,
  );
  compareContractValue(
    "artifact.config.providers",
    artifactConfig?.providers,
    source.providers,
  );
  compareContractValue(
    "artifact.config.tests",
    artifactConfig?.tests,
    contract.tests,
  );
  compareContractValue(
    "artifact.config.defaultTest",
    artifactConfig?.defaultTest,
    {
      ...source.defaultTest,
      vars: {},
      assert: [],
      metadata: {},
    },
  );
  compareContractValue(
    "artifact.config.metadata",
    artifactConfig?.metadata,
    source.metadata,
  );

  for (const [key, expected] of [
    ["tags", {}],
    ["env", {}],
    ["extensions", []],
    ["evaluateOptions", {}],
  ]) {
    compareContractValue(
      `artifact.config.${key}`,
      artifactConfig?.[key],
      expected,
    );
  }
  if (
    !Array.isArray(artifactConfig?.outputPath) ||
    artifactConfig.outputPath.length === 0 ||
    !artifactConfig.outputPath.every(
      (output) => typeof output === "string" && output.length > 0,
    ) ||
    !artifactConfig.outputPath.some(
      (output) => path.resolve(output) === path.resolve(resultsPath),
    )
  ) {
    addContractDifference("artifact.config.outputPath");
  }

  compareContractValue(
    "artifact.metadata.promptfooVersion",
    raw.metadata?.promptfooVersion,
    "0.121.18",
  );
  compareContractValue("artifact.shareableUrl", raw.shareableUrl, null);
  if (
    !hasExactKeys(raw.runtimeOptions, [
      "eventSource",
      "showProgressBar",
      "repeat",
      "maxConcurrency",
      "cache",
    ])
  ) {
    addContractDifference("artifact.runtimeOptions keys");
  }
  compareContractValue(
    "artifact.runtimeOptions.eventSource",
    raw.runtimeOptions?.eventSource,
    "cli",
  );
  compareContractValue(
    "artifact.runtimeOptions.showProgressBar",
    raw.runtimeOptions?.showProgressBar,
    true,
  );
  compareContractValue(
    "artifact.runtimeOptions.repeat",
    raw.runtimeOptions?.repeat,
    1,
  );
  compareContractValue(
    "artifact.runtimeOptions.maxConcurrency",
    raw.runtimeOptions?.maxConcurrency,
    contract.expectedRuntimeMaxConcurrency,
  );
  compareContractValue(
    "artifact.runtimeOptions.cache",
    raw.runtimeOptions?.cache,
    source.commandLineOptions.cache,
  );
}

function normalizeOptionalSessionVars(vars, contractPath) {
  if (!isPlainObject(vars)) {
    addContractDifference(contractPath);
    return undefined;
  }
  const normalized = { ...vars };
  if (Object.hasOwn(normalized, "sessionId")) {
    if (
      typeof normalized.sessionId !== "string" ||
      normalized.sessionId.trim().length === 0
    ) {
      addContractDifference(`${contractPath}.sessionId`);
      return undefined;
    }
    delete normalized.sessionId;
  }
  return normalized;
}

function normalizeResolvedProvider(
  provider,
  expectedProvider,
  contract,
  contractPath,
) {
  if (!isPlainObject(provider)) {
    addContractDifference(contractPath);
    return undefined;
  }

  if (hasExactKeys(provider, ["id", "label", "config"])) {
    return provider;
  }
  if (!hasExactKeys(provider, ["options", "label"])) {
    addContractDifference(contractPath);
    return undefined;
  }
  if (!hasExactKeys(provider.options, ["id", "config"])) {
    addContractDifference(`${contractPath}.options`);
    return undefined;
  }
  if (!isPlainObject(provider.options.config)) {
    addContractDifference(`${contractPath}.options.config`);
    return undefined;
  }

  const { basePath, ...semanticConfig } = provider.options.config;
  if (
    typeof basePath !== "string" ||
    path.resolve(basePath) !== path.resolve(contract.benchmarkDirectory)
  ) {
    addContractDifference(`${contractPath}.options.config.basePath`);
  }
  const normalized = {
    id: provider.options.id,
    label: provider.label,
    config: semanticConfig,
  };
  if (stableJson(normalized) !== stableJson(expectedProvider)) {
    addContractDifference(contractPath);
  }
  return normalized;
}

function normalizePromptConfig(
  config,
  expectedProvider,
  contract,
  contractPath,
) {
  if (
    !hasExactKeys(config, ["provider"]) ||
    !hasExactKeys(config.provider, ["text"])
  ) {
    addContractDifference(contractPath);
    return undefined;
  }
  const normalizedProvider = normalizeResolvedProvider(
    config.provider.text,
    expectedProvider,
    contract,
    `${contractPath}.provider.text`,
  );
  if (!normalizedProvider) {
    return undefined;
  }
  return { provider: { text: normalizedProvider } };
}

function renderExpectedPrompt(template, vars) {
  return template.replace(
    /\{\{\s*scenario_prompt\s*\}\}/g,
    String(vars.scenario_prompt),
  );
}

function validateMeasurementResultContract(
  result,
  resultIndex,
  expectedTest,
  expectedProvider,
  contract,
) {
  const resultPath = `artifact.results.results[${resultIndex}]`;
  const expectedGrader = contract.source.defaultTest.options.provider.text;

  compareContractValue(`${resultPath}.provider`, result.provider, {
    id: expectedProvider.id,
    label: expectedProvider.label,
  });

  const normalizedResultVars = normalizeOptionalSessionVars(
    result.vars,
    `${resultPath}.vars`,
  );
  if (normalizedResultVars) {
    compareContractValue(
      `${resultPath}.vars`,
      normalizedResultVars,
      expectedTest.vars,
    );
  }

  if (!isPlainObject(result.testCase)) {
    addContractDifference(`${resultPath}.testCase`);
  } else {
    const normalizedTestVars = normalizeOptionalSessionVars(
      result.testCase.vars,
      `${resultPath}.testCase.vars`,
    );
    const normalizedOptions = normalizePromptConfig(
      result.testCase.options,
      expectedGrader,
      contract,
      `${resultPath}.testCase.options`,
    );
    if (normalizedTestVars && normalizedOptions) {
      compareContractValue(
        `${resultPath}.testCase`,
        {
          ...result.testCase,
          vars: normalizedTestVars,
          options: normalizedOptions,
        },
        {
          ...expectedTest,
          options: {
            provider: {
              text: expectedGrader,
            },
          },
          metadata: {},
        },
      );
    }
  }

  if (!hasExactKeys(result.prompt, ["raw", "label", "config"])) {
    addContractDifference(`${resultPath}.prompt`);
  } else {
    const normalizedPromptConfig = normalizePromptConfig(
      result.prompt.config,
      expectedGrader,
      contract,
      `${resultPath}.prompt.config`,
    );
    if (normalizedPromptConfig) {
      compareContractValue(
        `${resultPath}.prompt`,
        {
          ...result.prompt,
          config: normalizedPromptConfig,
        },
        {
          raw: renderExpectedPrompt(
            contract.source.prompts[0],
            expectedTest.vars,
          ),
          label: contract.source.prompts[0],
          config: {
            provider: {
              text: expectedGrader,
            },
          },
        },
      );
    }
  }
}

if (expectedMeasurementContract) {
  validatePersistedMeasurementConfig(expectedMeasurementContract);
}

function indexTestsByMeasurementKey(tests) {
  const indexed = new Map();
  for (const testCase of tests) {
    const key = measurementTestKey(testCase);
    if (!key) {
      continue;
    }
    if (!indexed.has(key)) {
      indexed.set(key, []);
    }
    indexed.get(key).push(testCase);
  }
  return indexed;
}

if (expectedMeasurementTests) {
  if (configuredTests.length === 0) {
    measurementFailures.push(
      "expected measurement contract requires persisted configured tests",
    );
  }

  const expectedByKey = indexTestsByMeasurementKey(expectedMeasurementTests);
  const configuredByKey = indexTestsByMeasurementKey(configuredTests);
  const expectedProviderByLabel = new Map(
    expectedMeasurementContract.source.providers.map((provider) => [
      provider.label,
      provider,
    ]),
  );

  for (const [key, expectedEntries] of expectedByKey) {
    if (expectedEntries.length !== 1) {
      measurementFailures.push(
        `${key}: canonical measurement contract contains duplicate configured tests`,
      );
      continue;
    }

    const configuredEntries = configuredByKey.get(key) || [];
    if (configuredEntries.length === 0) {
      measurementFailures.push(
        `${key}: missing expected configured measurement test`,
      );
      continue;
    }
    if (configuredEntries.length > 1) {
      measurementFailures.push(`${key}: duplicate configured measurement test`);
      continue;
    }

    const expectedContract = stableJson(expectedEntries[0]);
    const configuredContract = stableJson(configuredEntries[0]);
    if (configuredContract !== expectedContract) {
      measurementFailures.push(
        `${key}: persisted configured measurement test differs from canonical contract`,
      );
    }
  }

  for (const [index, testCase] of configuredTests.entries()) {
    const key = measurementTestKey(testCase);
    if (!key || !hasMeasurementMetadata(testCase?.vars || {})) {
      measurementFailures.push(
        `configured test ${index + 1}: unexpected unmarked configured test`,
      );
    } else if (!expectedByKey.has(key)) {
      measurementFailures.push(
        `${key}: unexpected configured measurement test`,
      );
    }
  }

  const expectedRows = new Map();
  for (const testCase of expectedMeasurementTests) {
    const key = measurementTestKey(testCase);
    for (const label of Array.isArray(testCase?.providers)
      ? testCase.providers
      : []) {
      expectedRows.set(`${key}::${label}`, {
        id: testCase.vars.case_id,
        sampleIndex: testCase.vars.sample_index ?? testCase.vars.sampleIndex,
        label,
      });
    }
  }

  const resultRowCounts = new Map();
  for (const [index, result] of results.entries()) {
    const vars = resultVars(result);
    if (!hasMeasurementMetadata(vars)) {
      measurementFailures.push(
        `result ${index + 1}: unexpected unmarked result`,
      );
      continue;
    }

    const id = vars.case_id;
    const sampleIndex = vars.sample_index ?? vars.sampleIndex;
    const label = result.provider?.label;
    const expectedTestEntries =
      expectedByKey.get(`${id}::${sampleIndex}`) ?? [];
    const expectedProvider = expectedProviderByLabel.get(label);
    if (expectedTestEntries.length === 1 && expectedProvider) {
      validateMeasurementResultContract(
        result,
        index,
        expectedTestEntries[0],
        expectedProvider,
        expectedMeasurementContract,
      );
    }
    const rowKey = `${id}::${sampleIndex}::${label}`;
    if (!expectedRows.has(rowKey)) {
      measurementFailures.push(
        `${id}/${label}/sample-${sampleIndex}: unexpected result row`,
      );
      continue;
    }
    resultRowCounts.set(rowKey, (resultRowCounts.get(rowKey) || 0) + 1);
  }

  for (const [rowKey, expectedRow] of expectedRows) {
    const count = resultRowCounts.get(rowKey) || 0;
    if (count === 0) {
      measurementFailures.push(
        `${expectedRow.id}: missing expected result for provider ${expectedRow.label}, sample ${expectedRow.sampleIndex}`,
      );
    } else if (count > 1) {
      measurementFailures.push(
        `${expectedRow.id}: duplicate result for provider ${expectedRow.label}, sample ${expectedRow.sampleIndex}`,
      );
    }
  }
}

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
