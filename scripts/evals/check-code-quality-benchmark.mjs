#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import {
  VerifierCompositionError,
  verifierComposition,
} from "../../evals/benchmarks/downstream-code-quality/verifiers/verifier-composition.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const benchmarkDirectory = path.join(
  root,
  "evals/benchmarks/downstream-code-quality",
);
const contract = JSON.parse(
  fs.readFileSync(path.join(benchmarkDirectory, "benchmark.json"), "utf8"),
);
const require = createRequire(import.meta.url);
const { inputHashFor, promptFor } = require(
  path.join(benchmarkDirectory, "benchmark-inputs.cjs"),
);
const { loadRuntimeManifest } = require(
  path.join(benchmarkDirectory, "runtime-manifest.cjs"),
);

const runMarkerName = ".ai-plugins-code-quality-run-root";
const runMarkerContents = "ai-plugins downstream code-quality run root\n";
const sha256Pattern = /^[0-9a-f]{64}$/;
const oidPattern = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const versionPattern =
  /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const modelPattern = /^[A-Za-z0-9][A-Za-z0-9._:/+-]{0,127}$/;
const maximumResultsBytes = 64 * 1024 * 1024;
const maximumArtifactBytes = 128 * 1024;
const maximumManifestBytes = 1024 * 1024;
const maximumRawTraceBytes = 16 * 1024 * 1024;
const maximumTraceItems = 4_096;
const maximumResults = 64;
const expectedTurns = 9;
const skillActivationEvidence =
  "codex-turn-successful-command-path-references";
const artifactKeys = [
  "baselineOid",
  "benchmarkId",
  "caseId",
  "changeEvidence",
  "compositionHash",
  "conditionId",
  "contractSha256",
  "fixtureDigest",
  "gates",
  "inputHash",
  "matrixHash",
  "outcomeClass",
  "pass",
  "promotionEligible",
  "providerLabel",
  "runId",
  "runtimeManifestSha256",
  "sampleIndex",
  "schemaVersion",
  "scoringMode",
  "taskType",
  "trustedFixtureSha256",
  "verifier",
  "verifierCompositionSha256",
  "workspaceManifestSha256",
];
const gateNames = [
  "baseline-regression-replay",
  "black-box-behavior",
  "clippy",
  "diff-scope",
  "format",
  "regression-tests",
  "safety",
  "source-rebuild",
];
const changeEvidenceNames = [
  "addedFileCount",
  "candidateTreeSha256",
  "changedFileCount",
  "deletedFileCount",
  "diffSha256",
  "modifiedFileCount",
  "sourceByteCount",
  "sourceFileCount",
];
const provenanceKeys = [
  "benchmarkId",
  "boundarySha256",
  "codexBinarySha256",
  "codexSdkVersion",
  "codexVersion",
  "contractSha256",
  "matrixHash",
  "model",
  "nodeBinarySha256",
  "nodeVersion",
  "packageLockSha256",
  "promptfooVersion",
  "reasoningEffort",
  "runId",
  "runtimeManifestSha256",
  "schemaVersion",
  "toolchainCompositionSha256",
  "workspaceManifestSha256",
];
const bindingFields = [
  ["baseline_oid", "baselineOid"],
  ["case_id", "caseId"],
  ["condition_id", "mode"],
  ["fixture_digest", "fixtureDigest"],
  ["sample_index", "sample"],
  ["task_type", "taskType"],
  ["workspace", "workspace"],
  ["available_skills", "availableSkills"],
  ["codex_home", "codexHome"],
  ["codex_tmp", "codexTmp"],
  ["composition_hash", "compositionHash"],
  ["contract_sha256", "contractSha256"],
  ["input_hash", "inputHash"],
  ["matrix_hash", "matrixHash"],
  ["run_id", "runId"],
  ["workspace_manifest_sha256", "workspaceManifestSha256"],
];

class CheckFailure extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function hasExactKeys(value, keys) {
  return (
    isPlainObject(value) &&
    JSON.stringify(Object.keys(value).sort()) ===
      JSON.stringify([...keys].sort())
  );
}

function isFiniteNonnegative(value, maximum = Number.MAX_SAFE_INTEGER) {
  return (
    typeof value === "number" &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= maximum
  );
}

function isBoundedInteger(value, maximum = Number.MAX_SAFE_INTEGER) {
  return Number.isSafeInteger(value) && value >= 0 && value <= maximum;
}

function isStrictDescendant(ancestor, descendant) {
  const relative = path.relative(ancestor, descendant);
  return (
    relative !== "" &&
    relative !== ".." &&
    !relative.startsWith(`..${path.sep}`) &&
    !path.isAbsolute(relative)
  );
}

function pathsOverlap(first, second) {
  return (
    first === second ||
    isStrictDescendant(first, second) ||
    isStrictDescendant(second, first)
  );
}

function parseArguments(argv) {
  const names = new Set([
    "--artifacts",
    "--output",
    "--provenance",
    "--results",
    "--runtime-manifest",
  ]);
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (
      !names.has(name) ||
      !value ||
      values.has(name) ||
      !path.isAbsolute(value) ||
      path.resolve(value) !== value
    ) {
      throw new CheckFailure("invalid-arguments");
    }
    values.set(name, value);
  }
  if (argv.length !== names.size * 2 || values.size !== names.size) {
    throw new CheckFailure("invalid-arguments");
  }
  return {
    artifactRoot: values.get("--artifacts"),
    output: values.get("--output"),
    provenanceFile: values.get("--provenance"),
    resultsFile: values.get("--results"),
    runtimeManifestFile: values.get("--runtime-manifest"),
  };
}

function assertPrivateDirectory(directory, code) {
  const stat = fs.lstatSync(directory, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isDirectory() ||
    stat.isSymbolicLink() ||
    (stat.mode & 0o077) !== 0 ||
    fs.realpathSync(directory) !== directory
  ) {
    throw new CheckFailure(code);
  }
  return stat;
}

function readPrivateFile(file, maximumBytes, code) {
  const before = fs.lstatSync(file, { throwIfNoEntry: false });
  if (
    !before ||
    !before.isFile() ||
    before.isSymbolicLink() ||
    (before.mode & 0o077) !== 0 ||
    before.nlink !== 1 ||
    before.size < 2 ||
    before.size > maximumBytes ||
    fs.realpathSync(file) !== file
  ) {
    throw new CheckFailure(code);
  }
  let descriptor;
  try {
    descriptor = fs.openSync(
      file,
      fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW,
    );
  } catch {
    throw new CheckFailure(code);
  }
  try {
    const after = fs.fstatSync(descriptor);
    if (
      !after.isFile() ||
      after.nlink !== 1 ||
      before.dev !== after.dev ||
      before.ino !== after.ino ||
      before.size !== after.size
    ) {
      throw new CheckFailure(code);
    }
    const bytes = fs.readFileSync(descriptor);
    if (bytes.length !== after.size) throw new CheckFailure(code);
    return bytes;
  } catch (error) {
    if (error instanceof CheckFailure) throw error;
    throw new CheckFailure(code);
  } finally {
    fs.closeSync(descriptor);
  }
}

function parseJson(bytes, code) {
  try {
    return JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new CheckFailure(code);
  }
}

function loadTrustedVerifierCompositionSha256() {
  try {
    return verifierComposition().sha256;
  } catch (error) {
    if (error instanceof CheckFailure) throw error;
    if (error instanceof VerifierCompositionError) {
      throw new CheckFailure("trusted-verifier-invalid");
    }
    throw new CheckFailure("trusted-verifier-invalid");
  }
}

function assertRunLayout({
  artifactRoot,
  provenanceFile,
  resultsFile,
  runtimeManifestFile,
}) {
  if (
    path.basename(resultsFile) !== "results.json" ||
    path.basename(path.dirname(resultsFile)) !== "raw"
  ) {
    throw new CheckFailure("run-layout-invalid");
  }
  const runRoot = path.dirname(path.dirname(resultsFile));
  if (
    artifactRoot !== path.join(runRoot, "artifacts") ||
    provenanceFile !== path.join(runRoot, "provenance.json") ||
    runtimeManifestFile !==
      path.join(runRoot, "host-tmp", "runtime", "manifest.json")
  ) {
    throw new CheckFailure("run-layout-invalid");
  }
  const temporaryRoot = fs.realpathSync(os.tmpdir());
  if (!isStrictDescendant(temporaryRoot, runRoot)) {
    throw new CheckFailure("run-layout-invalid");
  }
  assertPrivateDirectory(runRoot, "run-layout-invalid");
  assertPrivateDirectory(path.join(runRoot, "raw"), "run-layout-invalid");
  assertPrivateDirectory(artifactRoot, "run-layout-invalid");
  assertPrivateDirectory(
    path.join(runRoot, "host-tmp", "workspaces"),
    "run-layout-invalid",
  );
  const marker = readPrivateFile(
    path.join(runRoot, runMarkerName),
    256,
    "run-layout-invalid",
  );
  if (!marker.equals(Buffer.from(runMarkerContents))) {
    throw new CheckFailure("run-layout-invalid");
  }
  return runRoot;
}

function loadBoundRuntime(runtimeManifestFile, expectedWorkspaceManifest) {
  const runtimeBytes = readPrivateFile(
    runtimeManifestFile,
    maximumManifestBytes,
    "runtime-manifest-invalid",
  );
  const preliminary = parseJson(runtimeBytes, "runtime-manifest-invalid");
  if (
    !isPlainObject(preliminary) ||
    typeof preliminary.workspaceManifest !== "string" ||
    !path.isAbsolute(preliminary.workspaceManifest)
  ) {
    throw new CheckFailure("runtime-manifest-invalid");
  }
  if (preliminary.workspaceManifest !== expectedWorkspaceManifest) {
    throw new CheckFailure("run-layout-invalid");
  }
  const saved = new Map(
    [
      "CODE_QUALITY_RUNTIME_MANIFEST",
      "CODE_QUALITY_WORKSPACE_MANIFEST",
      "EVAL_CASE_FILTER",
      "EVAL_SAMPLES",
    ].map((name) => [name, process.env[name]]),
  );
  process.env.CODE_QUALITY_RUNTIME_MANIFEST = runtimeManifestFile;
  process.env.CODE_QUALITY_WORKSPACE_MANIFEST = preliminary.workspaceManifest;
  process.env.EVAL_CASE_FILTER = "rust-cli-feature";
  process.env.EVAL_SAMPLES = "3";
  try {
    const state = loadRuntimeManifest({
      inspectGit: false,
      phase: "post-turn",
    });
    if (
      state.runtimeManifestSha256 !== sha256(runtimeBytes) ||
      state.rows.length !== expectedTurns ||
      state.workspaceState.manifest.sampleCount !== 3 ||
      state.rows.some(
        (row) =>
          row.caseId !== "rust-cli-feature" || row.sample < 1 || row.sample > 3,
      )
    ) {
      throw new CheckFailure("runtime-matrix-noncanonical");
    }
    return { ...state, runtimeBytes };
  } catch (error) {
    if (error instanceof CheckFailure) throw error;
    throw new CheckFailure("runtime-manifest-invalid");
  } finally {
    for (const [name, value] of saved) {
      if (value === undefined) delete process.env[name];
      else process.env[name] = value;
    }
  }
}

function loadProvenance(provenanceFile, runtimeState) {
  const value = parseJson(
    readPrivateFile(provenanceFile, 32 * 1024, "provenance-invalid"),
    "provenance-invalid",
  );
  const { manifest } = runtimeState;
  if (
    !hasExactKeys(value, provenanceKeys) ||
    value.schemaVersion !== 1 ||
    value.benchmarkId !== contract.id ||
    value.runId !== manifest.runId ||
    value.contractSha256 !== manifest.contractSha256 ||
    value.workspaceManifestSha256 !== manifest.workspaceManifestSha256 ||
    value.runtimeManifestSha256 !== runtimeState.runtimeManifestSha256 ||
    value.matrixHash !== manifest.matrixHash ||
    value.model !== contract.provider.model ||
    value.reasoningEffort !== contract.provider.reasoningEffort ||
    !versionPattern.test(value.codexVersion) ||
    !versionPattern.test(value.codexSdkVersion) ||
    !versionPattern.test(value.nodeVersion) ||
    !versionPattern.test(value.promptfooVersion) ||
    !modelPattern.test(value.model) ||
    !["low", "medium", "high", "xhigh"].includes(value.reasoningEffort) ||
    !sha256Pattern.test(value.codexBinarySha256) ||
    !sha256Pattern.test(value.nodeBinarySha256) ||
    !sha256Pattern.test(value.packageLockSha256) ||
    !sha256Pattern.test(value.boundarySha256) ||
    !sha256Pattern.test(value.toolchainCompositionSha256)
  ) {
    throw new CheckFailure("provenance-invalid");
  }
  return value;
}

function expectedArtifactPaths(rows) {
  const directories = new Set();
  const files = new Map();
  for (const row of rows) {
    const caseDirectory = row.caseId;
    const sampleDirectory = path.join(caseDirectory, `sample-${row.sample}`);
    directories.add(caseDirectory);
    directories.add(sampleDirectory);
    files.set(path.join(sampleDirectory, `${row.mode}.json`), row);
  }
  return { directories, files };
}

function inspectArtifactTree(artifactRoot, rows) {
  const expected = expectedArtifactPaths(rows);
  let entries = 0;

  function visit(directory, relativeDirectory) {
    let handle;
    try {
      handle = fs.opendirSync(directory);
    } catch {
      throw new CheckFailure("artifact-tree-invalid");
    }
    try {
      let entry;
      while ((entry = handle.readSync())) {
        entries += 1;
        if (entries > 64) throw new CheckFailure("artifact-tree-invalid");
        const candidate = path.join(directory, entry.name);
        const relative = relativeDirectory
          ? path.join(relativeDirectory, entry.name)
          : entry.name;
        const stat = fs.lstatSync(candidate, { throwIfNoEntry: false });
        if (!stat || stat.isSymbolicLink() || (stat.mode & 0o077) !== 0) {
          throw new CheckFailure("artifact-tree-invalid");
        }
        if (stat.isDirectory()) {
          if (!expected.directories.has(relative)) {
            throw new CheckFailure("artifact-tree-invalid");
          }
          visit(candidate, relative);
        } else if (stat.isFile()) {
          if (
            !expected.files.has(relative) ||
            stat.size < 2 ||
            stat.size > maximumArtifactBytes
          ) {
            throw new CheckFailure("artifact-tree-invalid");
          }
        } else {
          throw new CheckFailure("artifact-tree-invalid");
        }
      }
    } finally {
      handle.closeSync();
    }
  }

  visit(artifactRoot, "");
  return expected;
}

function validateChangeEvidence(value) {
  return (
    hasExactKeys(value, changeEvidenceNames) &&
    isBoundedInteger(value.sourceFileCount, 64) &&
    isBoundedInteger(value.sourceByteCount, 2 * 1024 * 1024) &&
    isBoundedInteger(value.addedFileCount, 64) &&
    isBoundedInteger(value.modifiedFileCount, 64) &&
    isBoundedInteger(value.deletedFileCount, 64) &&
    isBoundedInteger(value.changedFileCount, 128) &&
    value.changedFileCount ===
      value.addedFileCount + value.modifiedFileCount + value.deletedFileCount &&
    sha256Pattern.test(value.candidateTreeSha256) &&
    sha256Pattern.test(value.diffSha256)
  );
}

function validateArtifact(
  value,
  row,
  runtimeState,
  trustedVerifierCompositionSha256,
) {
  const expectedProvider = `openai-codex-sdk-${row.mode}`;
  const gatesAreValid =
    hasExactKeys(value?.gates, gateNames) &&
    Object.values(value.gates).every((gate) => typeof gate === "boolean");
  const allGatesPass =
    gatesAreValid && Object.values(value.gates).every(Boolean);
  const expectedOutcome = allGatesPass
    ? "pass"
    : value?.gates?.safety
      ? "candidate-failure"
      : "safety-failure";
  if (
    !hasExactKeys(value, artifactKeys) ||
    value.schemaVersion !== 1 ||
    value.benchmarkId !== contract.id ||
    value.caseId !== row.caseId ||
    value.taskType !== row.taskType ||
    value.conditionId !== row.mode ||
    value.sampleIndex !== row.sample ||
    value.providerLabel !== expectedProvider ||
    value.baselineOid !== row.baselineOid ||
    !oidPattern.test(value.baselineOid) ||
    value.runId !== row.runId ||
    value.contractSha256 !== row.contractSha256 ||
    value.workspaceManifestSha256 !== row.workspaceManifestSha256 ||
    value.runtimeManifestSha256 !== runtimeState.runtimeManifestSha256 ||
    value.matrixHash !== row.matrixHash ||
    value.fixtureDigest !== row.fixtureDigest ||
    value.inputHash !== row.inputHash ||
    value.compositionHash !== row.compositionHash ||
    value.promotionEligible !== false ||
    value.scoringMode !== "trusted-source-rebuild" ||
    typeof value.pass !== "boolean" ||
    !["pass", "candidate-failure", "safety-failure"].includes(
      value.outcomeClass,
    ) ||
    !gatesAreValid ||
    value.pass !== allGatesPass ||
    value.outcomeClass !== expectedOutcome ||
    value.trustedFixtureSha256 !== row.fixtureDigest ||
    value.verifierCompositionSha256 !== trustedVerifierCompositionSha256 ||
    value.verifier !== "expense-report-trusted-source" ||
    !validateChangeEvidence(value.changeEvidence)
  ) {
    return undefined;
  }
  return value;
}

function loadArtifacts(
  artifactRoot,
  rows,
  runtimeState,
  trustedVerifierCompositionSha256,
) {
  const expected = inspectArtifactTree(artifactRoot, rows);
  const artifacts = new Map();
  for (const [relative, row] of expected.files) {
    const file = path.join(artifactRoot, relative);
    if (!fs.existsSync(file)) continue;
    let value;
    try {
      value = parseJson(
        readPrivateFile(file, maximumArtifactBytes, "artifact-invalid"),
        "artifact-invalid",
      );
    } catch {
      artifacts.set(rowKey(row), { present: true, valid: false });
      continue;
    }
    const artifact = validateArtifact(
      value,
      row,
      runtimeState,
      trustedVerifierCompositionSha256,
    );
    artifacts.set(rowKey(row), {
      artifact,
      present: true,
      valid: Boolean(artifact),
    });
  }
  return artifacts;
}

function rowKey(row) {
  return `${row.caseId}\0${row.sample}\0${row.mode}`;
}

function rawRowKey(result) {
  const vars = result?.vars;
  if (
    !isPlainObject(vars) ||
    !identifierPattern.test(vars.case_id || "") ||
    !identifierPattern.test(vars.condition_id || "") ||
    !Number.isInteger(vars.sample_index)
  ) {
    return undefined;
  }
  return `${vars.case_id}\0${vars.sample_index}\0${vars.condition_id}`;
}

function sameValue(first, second) {
  if (Array.isArray(first) || Array.isArray(second)) {
    return JSON.stringify(first) === JSON.stringify(second);
  }
  return first === second;
}

function validateRawBinding(result, row, runtimeState) {
  if (
    !isPlainObject(result) ||
    !isPlainObject(result.vars) ||
    !isPlainObject(result.testCase) ||
    !isPlainObject(result.testCase.vars)
  ) {
    return false;
  }
  const variableSets = [result.vars, result.testCase.vars];
  for (const vars of variableSets) {
    for (const [variable, field] of bindingFields) {
      if (!sameValue(vars[variable], row[field])) return false;
    }
  }
  const providerLabel = `openai-codex-sdk-${row.mode}`;
  const scenarioPrompt = promptFor(row);
  return (
    variableSets.every(
      (vars) =>
        vars.runtime_manifest_sha256 === runtimeState.runtimeManifestSha256 &&
        vars.expected_provider_label === providerLabel &&
        vars.benchmark_expected_samples === 3 &&
        vars.min_pass_rate === 0 &&
        vars.value_gate_mode === "measurement" &&
        vars.scenario_prompt === scenarioPrompt,
    ) &&
    inputHashFor(row, contract.id) === row.inputHash &&
    result.provider?.id === contract.provider.id &&
    result.provider?.label === providerLabel
  );
}

function sanitizeUsageObject(value, allowAssertions) {
  if (!isPlainObject(value)) return undefined;
  const result = {};
  for (const name of [
    "prompt",
    "completion",
    "cached",
    "total",
    "numRequests",
  ]) {
    if (value[name] === undefined) continue;
    if (!isBoundedInteger(value[name])) return undefined;
    result[name] = value[name];
  }
  if (value.completionDetails !== undefined) {
    if (!isPlainObject(value.completionDetails)) return undefined;
    const details = {};
    for (const name of [
      "reasoning",
      "acceptedPrediction",
      "rejectedPrediction",
      "cacheReadInputTokens",
      "cacheCreationInputTokens",
    ]) {
      if (value.completionDetails[name] === undefined) continue;
      if (!isBoundedInteger(value.completionDetails[name])) return undefined;
      details[name] = value.completionDetails[name];
    }
    result.completionDetails = details;
  }
  if (allowAssertions && value.assertions !== undefined) {
    const assertions = sanitizeUsageObject(value.assertions, false);
    if (!assertions) return undefined;
    result.assertions = assertions;
  }
  if (
    !Object.hasOwn(result, "total") &&
    !Object.hasOwn(result, "prompt") &&
    !Object.hasOwn(result, "completion")
  ) {
    return undefined;
  }
  return result;
}

function sanitizeMetrics(result) {
  if (
    !isFiniteNonnegative(result.latencyMs, 24 * 60 * 60 * 1000) ||
    !isFiniteNonnegative(result.cost, 1_000)
  ) {
    return undefined;
  }
  const tokenUsage = sanitizeUsageObject(result.tokenUsage, true);
  if (!tokenUsage) return undefined;
  return { cost: result.cost, latencyMs: result.latencyMs, tokenUsage };
}

function isValidUsage(value) {
  if (value === null) return true;
  const keys = [
    "cached_input_tokens",
    "input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
  ];
  return (
    hasExactKeys(value, keys) &&
    keys.every((key) => isBoundedInteger(value[key]))
  );
}

function canonicalRegularFile(file) {
  try {
    const stat = fs.lstatSync(file, { throwIfNoEntry: false });
    return Boolean(
      stat &&
      stat.isFile() &&
      !stat.isSymbolicLink() &&
      fs.realpathSync(file) === file,
    );
  } catch {
    return false;
  }
}

function activationForPath(sandboxPath, row, allowed) {
  const systemMatch = sandboxPath.match(
    /^\/runtime\/codex-home\/skills\/\.system\/([a-z0-9]+(?:-[a-z0-9]+)*)\/SKILL\.md$/,
  );
  if (systemMatch) {
    const qualifiedName = `codex-system:${systemMatch[1]}`;
    const physicalPath = path.join(
      row.codexHome,
      "skills/.system",
      systemMatch[1],
      "SKILL.md",
    );
    return allowed.has(qualifiedName) && canonicalRegularFile(physicalPath)
      ? qualifiedName
      : undefined;
  }

  const marketplaceMatch = sandboxPath.match(
    /^\/runtime\/codex-home\/plugins\/cache\/ai-plugins\/([a-z0-9]+(?:-[a-z0-9]+)*)\/([^/\s]+)\/skills\/([a-z0-9]+(?:-[a-z0-9]+)*)\/SKILL\.md$/,
  );
  if (!marketplaceMatch || !versionPattern.test(marketplaceMatch[2])) {
    return undefined;
  }
  const [, plugin, version, skill] = marketplaceMatch;
  const qualifiedName = `${plugin}:${skill}`;
  const physicalPath = path.join(
    row.codexHome,
    "plugins/cache/ai-plugins",
    plugin,
    version,
    "skills",
    skill,
    "SKILL.md",
  );
  return allowed.has(qualifiedName) && canonicalRegularFile(physicalPath)
    ? qualifiedName
    : undefined;
}

function skillActivations(result, row) {
  const raw = result?.response?.raw;
  if (
    typeof raw !== "string" ||
    raw.length === 0 ||
    Buffer.byteLength(raw, "utf8") > maximumRawTraceBytes
  ) {
    return { available: false, activations: [] };
  }

  let trace;
  try {
    trace = JSON.parse(raw);
  } catch {
    return { available: false, activations: [] };
  }
  if (
    !hasExactKeys(trace, ["finalResponse", "items", "usage"]) ||
    typeof trace.finalResponse !== "string" ||
    !Array.isArray(trace.items) ||
    trace.items.length > maximumTraceItems ||
    !isValidUsage(trace.usage)
  ) {
    return { available: false, activations: [] };
  }

  const allowedItemTypes = new Set([
    "agent_message",
    "command_execution",
    "error",
    "file_change",
    "mcp_tool_call",
    "reasoning",
    "todo_list",
    "web_search",
  ]);
  const commands = [];
  for (const item of trace.items) {
    if (
      !isPlainObject(item) ||
      typeof item.id !== "string" ||
      item.id.length === 0 ||
      item.id.length > 512 ||
      !allowedItemTypes.has(item.type)
    ) {
      return { available: false, activations: [] };
    }
    if (item.type !== "command_execution") continue;
    if (
      typeof item.command !== "string" ||
      item.command.length === 0 ||
      item.command.length > 64 * 1024 ||
      typeof item.aggregated_output !== "string" ||
      item.aggregated_output.length > maximumRawTraceBytes ||
      !["in_progress", "completed", "failed"].includes(item.status) ||
      (item.exit_code !== undefined &&
        (!Number.isSafeInteger(item.exit_code) ||
          item.exit_code < -1 ||
          item.exit_code > 255)) ||
      (item.status === "completed" && item.exit_code === undefined)
    ) {
      return { available: false, activations: [] };
    }
    if (item.status === "completed" && item.exit_code === 0) {
      commands.push(item.command);
    }
  }

  const allowed = new Set(row.availableSkills);
  const activations = new Set();
  for (const command of commands) {
    for (const rawToken of command.split(/\s+/)) {
      const token = rawToken
        .replace(/^[`"'([{<]+|[`"',;:)\]}>]+$/g, "")
        .trim()
        .replaceAll("\\", "/");
      if (!token) continue;
      const activation = activationForPath(token, row, allowed);
      if (activation) activations.add(activation);
    }
  }
  return { available: true, activations: [...activations].sort() };
}

function classifiedMissingArtifact(result) {
  const candidates = [
    result?.error,
    result?.providerError,
    result?.response?.error,
    result?.response?.providerError,
    result?.gradingResult?.reason,
    result?.gradingResult?.error,
    result?.gradingResult?.providerError,
  ];
  for (const candidate of candidates) {
    if (typeof candidate !== "string") continue;
    const assertion = candidate.match(
      /^(?:Error: )?(provenance-failure|operational-failure|safety-failure):[a-z0-9-]+$/,
    );
    if (assertion) return assertion[1];
    const boundary = candidate.match(
      /CODE_QUALITY_BOUNDARY_ERROR:(configuration|integrity|invocation|runtime|safety|timeout):[a-z0-9-]+/,
    );
    if (boundary) {
      if (boundary[1] === "safety") return "safety-failure";
      if (["integrity", "invocation"].includes(boundary[1])) {
        return "provenance-failure";
      }
      return "operational-failure";
    }
  }
  const providerSpecific = [
    result?.providerError,
    result?.response?.error,
    result?.response?.providerError,
    result?.gradingResult?.providerError,
  ];
  if (
    providerSpecific.some(
      (value) => typeof value === "string" && value.trim().length > 0,
    ) ||
    (typeof result?.error === "string" &&
      /^(?:Error: )?(?:error calling provider|provider (?:error|unavailable)|api (?:error|request failed))(?::|\b)/i.test(
        result.error,
      ))
  ) {
    return "provider-failure";
  }
  return "operational-failure";
}

function rawAndArtifactAgree(result, artifact) {
  if (
    typeof result.success !== "boolean" ||
    !isPlainObject(result.gradingResult) ||
    typeof result.gradingResult.pass !== "boolean"
  ) {
    return false;
  }
  return (
    result.success === artifact.pass &&
    result.gradingResult.pass === artifact.pass &&
    (artifact.pass ? result.failureReason === 0 : result.failureReason === 1)
  );
}

function buildRun(row, rawEntries, artifactEntry, runtimeState) {
  const providerLabel = `openai-codex-sdk-${row.mode}`;
  const base = {
    caseId: row.caseId,
    taskType: row.taskType,
    conditionId: row.mode,
    sampleIndex: row.sample,
    providerLabel,
    baselineOid: row.baselineOid,
    fixtureDigest: row.fixtureDigest,
    inputHash: row.inputHash,
    compositionHash: row.compositionHash,
  };
  if (rawEntries.length === 0) {
    return {
      ...base,
      complete: false,
      pass: false,
      outcomeClass: artifactEntry?.present
        ? "provenance-failure"
        : "operational-failure",
      skillActivations: [],
    };
  }
  if (rawEntries.length !== 1) {
    return {
      ...base,
      complete: false,
      pass: false,
      outcomeClass: "provenance-failure",
      skillActivations: [],
    };
  }
  const result = rawEntries[0];
  if (!validateRawBinding(result, row, runtimeState)) {
    return {
      ...base,
      complete: false,
      pass: false,
      outcomeClass: "provenance-failure",
      skillActivations: [],
    };
  }
  const metrics = sanitizeMetrics(result);
  const activationTelemetry = skillActivations(result, row);
  const activationFields = activationTelemetry.available
    ? {
        skillActivationEvidence,
        skillActivations: activationTelemetry.activations,
      }
    : { skillActivations: [] };
  if (!artifactEntry?.present) {
    return {
      ...base,
      complete: Boolean(metrics),
      pass: false,
      outcomeClass: classifiedMissingArtifact(result),
      ...(metrics ? { metrics } : {}),
      ...activationFields,
    };
  }
  if (
    !artifactEntry.valid ||
    !rawAndArtifactAgree(result, artifactEntry.artifact)
  ) {
    return {
      ...base,
      complete: Boolean(metrics),
      pass: false,
      outcomeClass: "provenance-failure",
      ...(metrics ? { metrics } : {}),
      ...activationFields,
    };
  }
  if (!activationTelemetry.available) {
    return {
      ...base,
      complete: Boolean(metrics),
      pass: false,
      outcomeClass: "provenance-failure",
      ...(metrics ? { metrics } : {}),
      skillActivations: [],
    };
  }
  const artifact = artifactEntry.artifact;
  return {
    ...base,
    complete: Boolean(metrics),
    pass: artifact.pass,
    outcomeClass: artifact.outcomeClass,
    ...(metrics ? { metrics } : {}),
    ...activationFields,
    trustedFixtureSha256: artifact.trustedFixtureSha256,
    verifierCompositionSha256: artifact.verifierCompositionSha256,
    verifier: artifact.verifier,
    changeEvidence: artifact.changeEvidence,
    gates: artifact.gates,
  };
}

function rawResultsArray(raw) {
  const summary = raw?.results?.version === 3 ? raw.results : raw;
  if (
    !isPlainObject(summary) ||
    summary.version !== 3 ||
    !Array.isArray(summary.results) ||
    summary.results.length > maximumResults
  ) {
    throw new CheckFailure("raw-results-schema-invalid");
  }
  return summary.results;
}

function buildSanitized(runtimeState, provenance, artifacts, rawResults) {
  const expected = new Map(runtimeState.rows.map((row) => [rowKey(row), row]));
  const rawByKey = new Map([...expected.keys()].map((key) => [key, []]));
  let unexpectedResults = 0;
  for (const result of rawResults) {
    const key = rawRowKey(result);
    const bucket = key && rawByKey.get(key);
    if (!bucket) unexpectedResults += 1;
    else bucket.push(result);
  }
  const duplicateResults = [...rawByKey.values()].reduce(
    (total, values) => total + Math.max(0, values.length - 1),
    0,
  );
  const missingResults = [...rawByKey.values()].filter(
    (values) => values.length === 0,
  ).length;
  const runs = runtimeState.rows.map((row) =>
    buildRun(
      row,
      rawByKey.get(rowKey(row)),
      artifacts.get(rowKey(row)),
      runtimeState,
    ),
  );
  const outcomeCounts = {
    pass: runs.filter((run) => run.outcomeClass === "pass").length,
    candidateFailure: runs.filter(
      (run) => run.outcomeClass === "candidate-failure",
    ).length,
    safetyFailure: runs.filter((run) => run.outcomeClass === "safety-failure")
      .length,
    operationalFailure: runs.filter(
      (run) => run.outcomeClass === "operational-failure",
    ).length,
    provenanceFailure: runs.filter(
      (run) => run.outcomeClass === "provenance-failure",
    ).length,
    providerFailure: runs.filter(
      (run) => run.outcomeClass === "provider-failure",
    ).length,
  };
  const completeRuns = runs.filter((run) => run.complete).length;
  const diagnostics = {
    expectedRuns: expectedTurns,
    completeRuns,
    unexpectedResults,
    duplicateResults,
    missingResults,
    candidateFailuresAreMeasurementOutcomes: true,
    safetyFailures: outcomeCounts.safetyFailure,
    operationalFailures: outcomeCounts.operationalFailure,
    provenanceFailures: outcomeCounts.provenanceFailure + unexpectedResults,
    providerFailures: outcomeCounts.providerFailure,
    outcomes: outcomeCounts,
  };
  const aggregates = contract.conditions.map((condition) => {
    const conditionRuns = runs.filter(
      (run) => run.conditionId === condition.id,
    );
    const successCount = conditionRuns.filter(
      (run) => run.complete && run.pass,
    ).length;
    return {
      conditionId: condition.id,
      sampleCount: conditionRuns.length,
      successCount,
      successRate: successCount / conditionRuns.length,
      passAt3Capability: successCount > 0 ? 1 : 0,
      passPower3Reliability:
        successCount === conditionRuns.length && conditionRuns.length === 3
          ? 1
          : 0,
    };
  });
  const diagnosticEligible =
    runs.length === expectedTurns &&
    completeRuns === expectedTurns &&
    unexpectedResults === 0 &&
    duplicateResults === 0 &&
    missingResults === 0 &&
    outcomeCounts.safetyFailure === 0 &&
    outcomeCounts.operationalFailure === 0 &&
    outcomeCounts.provenanceFailure === 0 &&
    outcomeCounts.providerFailure === 0;
  return {
    schemaVersion: 1,
    benchmarkId: contract.id,
    promotionEligible: false,
    diagnosticEligible,
    provenance: { ...provenance },
    diagnostics,
    runs,
    aggregates,
  };
}

function assertOutputPath(output, runRoot, runtimeState) {
  if (path.extname(output) !== ".json") {
    throw new CheckFailure("output-path-invalid");
  }
  const outputDirectory = path.dirname(output);
  assertPrivateDirectory(outputDirectory, "output-path-invalid");
  const existing = fs.lstatSync(output, { throwIfNoEntry: false });
  if (existing) {
    if (existing.isFile() && !existing.isSymbolicLink()) {
      throw new CheckFailure("output-already-exists");
    }
    throw new CheckFailure("output-path-invalid");
  }
  if (
    pathsOverlap(output, runRoot) ||
    pathsOverlap(output, runtimeState.runtimeRoot)
  ) {
    throw new CheckFailure("output-path-invalid");
  }
}

function writeOutput(output, value) {
  const directory = path.dirname(output);
  const temporary = path.join(
    directory,
    `.code-quality-results.${process.pid}.${crypto.randomBytes(8).toString("hex")}.tmp`,
  );
  try {
    fs.writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, {
      flag: "wx",
      mode: 0o600,
    });
    try {
      fs.linkSync(temporary, output);
    } catch (error) {
      if (error?.code === "EEXIST") {
        throw new CheckFailure("output-already-exists");
      }
      throw error;
    }
  } catch (error) {
    if (error instanceof CheckFailure) throw error;
    throw new CheckFailure("output-write-failed");
  } finally {
    try {
      fs.unlinkSync(temporary);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw new CheckFailure("output-write-failed");
      }
    }
  }
}

function main() {
  try {
    const arguments_ = parseArguments(process.argv.slice(2));
    const runRoot = assertRunLayout(arguments_);
    const runtimeState = loadBoundRuntime(
      arguments_.runtimeManifestFile,
      path.join(runRoot, "host-tmp", "workspaces", "manifest.json"),
    );
    const provenance = loadProvenance(arguments_.provenanceFile, runtimeState);
    const verifierCompositionSha256 = loadTrustedVerifierCompositionSha256();
    const artifacts = loadArtifacts(
      arguments_.artifactRoot,
      runtimeState.rows,
      runtimeState,
      verifierCompositionSha256,
    );
    const raw = parseJson(
      readPrivateFile(
        arguments_.resultsFile,
        maximumResultsBytes,
        "raw-results-file-invalid",
      ),
      "raw-results-schema-invalid",
    );
    const sanitized = buildSanitized(
      runtimeState,
      provenance,
      artifacts,
      rawResultsArray(raw),
    );
    assertOutputPath(arguments_.output, runRoot, runtimeState);
    writeOutput(arguments_.output, sanitized);
    process.stdout.write("code-quality-results:written\n");
  } catch (error) {
    const failure =
      error instanceof CheckFailure
        ? error
        : new CheckFailure("internal-failure");
    process.stderr.write(`code-quality-results:${failure.code}\n`);
    process.exitCode = 2;
  }
}

main();
