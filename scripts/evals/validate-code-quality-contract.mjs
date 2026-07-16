#!/usr/bin/env node
import fs from "node:fs";
import { pathToFileURL } from "node:url";

const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const expectedConditions = [
  "no-plugins",
  "targeted-plugins",
  "full-marketplace",
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

export function validateBenchmarkContract(contract) {
  assertObject(contract, "benchmark contract");
  if (contract.schemaVersion !== 1) {
    throw new Error("benchmark contract schemaVersion must be 1");
  }
  assertIdentifier(contract.id, "benchmark id");
  if (
    !Number.isInteger(contract.sampleCount) ||
    contract.sampleCount < 1 ||
    contract.sampleCount > 10
  ) {
    throw new Error(
      "benchmark sampleCount must be an integer from 1 through 10",
    );
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
  assertCanonicalPluginList(contract.conditions[0].plugins, "no-plugins");
  if (contract.conditions[0].plugins.length !== 0) {
    throw new Error("no-plugins composition must be empty");
  }
  assertCanonicalPluginList(contract.conditions[1].plugins, "targeted-plugins");
  if (contract.conditions[1].plugins.length === 0) {
    throw new Error("targeted-plugins composition must not be empty");
  }
  if (contract.conditions[2].plugins !== "codex-marketplace-at-run-start") {
    throw new Error(
      "full-marketplace composition must resolve from the Codex marketplace at run start",
    );
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
