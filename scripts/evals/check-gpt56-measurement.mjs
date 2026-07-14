#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import {
  resolveExpectedMeasurementSource,
  resolveExpectedRuntimeMaxConcurrency,
  SourceContractError,
  validateMeasurementArtifact,
} from "./gpt56-measurement-contract.mjs";

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
    "usage: check-gpt56-measurement.mjs <results.json> [--expected-measurement-config <promptfooconfig.yaml>]",
  );
  process.exit(2);
}

function isPlainObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
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

  const source = resolveExpectedMeasurementSource(parsed, process.env);
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

  const expectedRuntimeMaxConcurrency = resolveExpectedRuntimeMaxConcurrency(
    source.commandLineOptions.maxConcurrency,
    process.env.PROMPTFOO_MAX_CONCURRENCY,
  );

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

const failures = validateMeasurementArtifact({
  artifact: raw,
  results,
  resultsPath,
  workingDirectory: process.cwd(),
  expectedContract: expectedMeasurementContract,
});

if (failures.length > 0) {
  console.error("GPT-5.6 measurement contract failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.error(
  `GPT-5.6 measurement contract passed for ${results.length} result(s).`,
);
