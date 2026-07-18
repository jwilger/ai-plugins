const { spawnSync } = require("node:child_process");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const { inputHashFor, promptFor } = require("../benchmark-inputs.cjs");
const { providerLabelFor } = require("../manifest.cjs");
const {
  loadRuntimeManifest,
  RuntimeManifestError,
} = require("../runtime-manifest.cjs");

const scorer = path.resolve(__dirname, "../verifiers/score-expense-report.mjs");
const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const sha256Pattern = /^[0-9a-f]{64}$/;
const gateNames = [
  "source-rebuild",
  "black-box-behavior",
  "regression-tests",
  "baseline-regression-replay",
  "format",
  "clippy",
  "diff-scope",
  "safety",
];
const changeEvidenceNames = [
  "sourceFileCount",
  "sourceByteCount",
  "addedFileCount",
  "modifiedFileCount",
  "deletedFileCount",
  "changedFileCount",
  "candidateTreeSha256",
  "diffSha256",
];

function classifiedError(kind, code) {
  return new Error(`${kind}:${code}`);
}

function providerLabel(context) {
  if (typeof context?.provider?.label === "string") {
    return context.provider.label;
  }
  if (typeof context?.provider?.id === "function") {
    return context.provider.id();
  }
  if (typeof context?.provider?.id === "string") {
    return context.provider.id;
  }
  return "";
}

function findWorkspace(context) {
  try {
    const vars = context?.vars;
    if (!vars || typeof vars !== "object") throw new Error("missing vars");
    const runtimeState = loadRuntimeManifest({
      inspectGit: false,
      phase: "post-turn",
    });
    const { rows } = runtimeState;
    const row = rows.find(
      (candidate) =>
        candidate.caseId === vars.case_id &&
        candidate.sample === vars.sample_index &&
        candidate.mode === vars.condition_id,
    );
    if (!row) throw new Error("missing binding");
    const expectedProvider = providerLabelFor(row.mode);
    const expectedScenarioPrompt = promptFor(row);
    const expectedInputHash = inputHashFor(row);
    if (
      vars.workspace !== row.workspace ||
      vars.codex_home !== row.codexHome ||
      vars.codex_tmp !== row.codexTmp ||
      vars.scenario_prompt !== expectedScenarioPrompt ||
      row.inputHash !== expectedInputHash ||
      vars.expected_provider_label !== expectedProvider ||
      providerLabel(context) !== expectedProvider
    ) {
      throw new Error("binding mismatch");
    }
    return {
      expectedProvider,
      outputRoot: verifierOutputRoot(row.workspace, runtimeState.runtimeRoot),
      row,
      runtimeManifestSha256: runtimeState.runtimeManifestSha256,
    };
  } catch (error) {
    if (
      error instanceof Error &&
      /^(operational-failure|provenance-failure):[a-z0-9-]+$/.test(
        error.message,
      )
    ) {
      throw error;
    }
    if (
      error instanceof RuntimeManifestError &&
      error.category === "operational"
    ) {
      throw classifiedError(
        "operational-failure",
        "runtime-manifest-unavailable",
      );
    }
    throw classifiedError("provenance-failure", "workspace-binding-invalid");
  }
}

function pathsOverlap(first, second) {
  const firstToSecond = path.relative(first, second);
  const secondToFirst = path.relative(second, first);
  const contains = (relative) =>
    relative === "" ||
    (relative !== ".." &&
      !relative.startsWith(`..${path.sep}`) &&
      !path.isAbsolute(relative));
  return contains(firstToSecond) || contains(secondToFirst);
}

function prospectiveRealPath(value) {
  const missing = [];
  let current = path.resolve(value);
  while (!fs.existsSync(current)) {
    const parent = path.dirname(current);
    if (parent === current) {
      throw classifiedError("operational-failure", "output-root-invalid");
    }
    missing.unshift(path.basename(current));
    current = parent;
  }
  return path.join(fs.realpathSync(current), ...missing);
}

function isPrivateFile(file, stat = fs.lstatSync(file)) {
  return (
    stat.isFile() &&
    !stat.isSymbolicLink() &&
    (stat.mode & 0o077) === 0 &&
    fs.realpathSync(file) === file
  );
}

function verifierOutputRoot(workspace, runtimeRoot) {
  const configured = process.env.CODE_QUALITY_VERIFIER_OUT_ROOT;
  if (!configured || !path.isAbsolute(configured)) {
    throw classifiedError("operational-failure", "output-root-missing");
  }
  const runRoot = path.dirname(path.dirname(runtimeRoot));
  const expected = path.join(runRoot, "artifacts");
  if (configured !== expected || path.resolve(configured) !== configured) {
    throw classifiedError("provenance-failure", "output-root-unbound");
  }
  const runRootStat = fs.lstatSync(runRoot, { throwIfNoEntry: false });
  const marker = path.join(runRoot, ".ai-plugins-code-quality-run-root");
  const markerStat = fs.lstatSync(marker, { throwIfNoEntry: false });
  if (
    !runRootStat ||
    !isPrivateDirectory(runRoot, runRootStat) ||
    !markerStat ||
    !isPrivateFile(marker, markerStat) ||
    fs.readFileSync(marker, "utf8") !==
      "ai-plugins downstream code-quality run root\n"
  ) {
    throw classifiedError("operational-failure", "run-root-invalid");
  }
  const outputRoot = prospectiveRealPath(configured);
  if (outputRoot !== path.resolve(configured)) {
    throw classifiedError("operational-failure", "output-root-invalid");
  }
  if (pathsOverlap(outputRoot, workspace)) {
    throw classifiedError("operational-failure", "output-root-overlap");
  }
  const existing = fs.lstatSync(outputRoot, { throwIfNoEntry: false });
  if (!existing || !isPrivateDirectory(outputRoot, existing)) {
    throw classifiedError("operational-failure", "output-root-invalid");
  }
  return outputRoot;
}

function isPrivateDirectory(directory, stat = fs.lstatSync(directory)) {
  return (
    stat.isDirectory() &&
    !stat.isSymbolicLink() &&
    (stat.mode & 0o077) === 0 &&
    fs.realpathSync(directory) === directory
  );
}

function ensurePrivateDirectory(directory, errorCode) {
  let stat = fs.lstatSync(directory, { throwIfNoEntry: false });
  if (!stat) {
    const parent = path.dirname(directory);
    const parentStat = fs.lstatSync(parent, { throwIfNoEntry: false });
    if (
      !parentStat ||
      !parentStat.isDirectory() ||
      parentStat.isSymbolicLink() ||
      fs.realpathSync(parent) !== parent
    ) {
      throw classifiedError("operational-failure", errorCode);
    }
    try {
      fs.mkdirSync(directory, { mode: 0o700 });
    } catch {
      throw classifiedError("operational-failure", errorCode);
    }
    stat = fs.lstatSync(directory, { throwIfNoEntry: false });
  }
  if (!stat || !isPrivateDirectory(directory, stat)) {
    throw classifiedError("operational-failure", errorCode);
  }
}

function validateChangeEvidence(evidence) {
  if (
    !evidence ||
    typeof evidence !== "object" ||
    Array.isArray(evidence) ||
    JSON.stringify(Object.keys(evidence)) !==
      JSON.stringify(changeEvidenceNames)
  ) {
    return false;
  }
  const boundedInteger = (name, maximum) =>
    Number.isInteger(evidence[name]) &&
    evidence[name] >= 0 &&
    evidence[name] <= maximum;
  return (
    boundedInteger("sourceFileCount", 64) &&
    boundedInteger("sourceByteCount", 2 * 1024 * 1024) &&
    boundedInteger("addedFileCount", 64) &&
    boundedInteger("modifiedFileCount", 64) &&
    boundedInteger("deletedFileCount", 64) &&
    boundedInteger("changedFileCount", 128) &&
    evidence.changedFileCount ===
      evidence.addedFileCount +
        evidence.modifiedFileCount +
        evidence.deletedFileCount &&
    sha256Pattern.test(evidence.candidateTreeSha256) &&
    sha256Pattern.test(evidence.diffSha256)
  );
}

function validateReport(report, row) {
  if (
    !report ||
    report.schemaVersion !== 1 ||
    report.verifier !== "expense-report-trusted-source" ||
    typeof report.pass !== "boolean" ||
    !["pass", "candidate-failure", "safety-failure"].includes(
      report.outcomeClass,
    ) ||
    !report.gates ||
    typeof report.gates !== "object" ||
    Array.isArray(report.gates) ||
    JSON.stringify(Object.keys(report.gates)) !== JSON.stringify(gateNames) ||
    Object.values(report.gates).some((value) => typeof value !== "boolean") ||
    !validateChangeEvidence(report.changeEvidence) ||
    report.trustedFixtureSha256 !== row.fixtureDigest ||
    !sha256Pattern.test(report.verifierCompositionSha256) ||
    report.pass !== Object.values(report.gates).every(Boolean) ||
    (report.pass
      ? report.outcomeClass !== "pass"
      : report.outcomeClass === "pass") ||
    (report.outcomeClass === "safety-failure") === report.gates.safety
  ) {
    throw classifiedError("operational-failure", "scorer-report-invalid");
  }
  return report;
}

function scorerFailure(result) {
  const match = String(result.stderr || "")
    .trim()
    .match(
      /^score-expense-report:(operational-failure|provenance-failure):([a-z0-9-]+)$/,
    );
  if (match) return classifiedError(match[1], match[2]);
  return classifiedError("operational-failure", "scorer-process-failed");
}

function runScorer(row) {
  const result = spawnSync(
    process.execPath,
    [
      scorer,
      "--workspace",
      row.workspace,
      "--baseline-oid",
      row.baselineOid,
      "--trusted-fixture-digest",
      row.fixtureDigest,
    ],
    {
      cwd: row.workspace,
      encoding: "utf8",
      env: {
        AI_PLUGINS_BWRAP_BIN: process.env.AI_PLUGINS_BWRAP_BIN,
        AI_PLUGINS_PRLIMIT_BIN: process.env.AI_PLUGINS_PRLIMIT_BIN,
        CODE_QUALITY_NIX_STORE_CLOSURE:
          process.env.CODE_QUALITY_NIX_STORE_CLOSURE,
        CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256:
          process.env.CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256,
        CODE_QUALITY_SYSTEMD_RUN_BIN: process.env.CODE_QUALITY_SYSTEMD_RUN_BIN,
        CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256:
          process.env.CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256,
        CODE_QUALITY_VERIFIER_TMP_ROOT:
          process.env.CODE_QUALITY_VERIFIER_TMP_ROOT,
        LANG: "C.UTF-8",
        LC_ALL: "C.UTF-8",
        PATH: process.env.PATH,
      },
      killSignal: "SIGKILL",
      maxBuffer: 256 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 10 * 60 * 1000,
    },
  );
  if (result.error || result.status !== 0) throw scorerFailure(result);
  let report;
  try {
    report = JSON.parse(result.stdout);
  } catch {
    throw classifiedError("operational-failure", "scorer-report-invalid");
  }
  return validateReport(report, row);
}

function writeArtifact({
  expectedProvider,
  outputRoot,
  report,
  row,
  runtimeManifestSha256,
}) {
  for (const value of [row.caseId, row.mode]) {
    if (!identifierPattern.test(value)) {
      throw classifiedError("provenance-failure", "artifact-id-invalid");
    }
  }
  const artifactDirectory = path.join(
    outputRoot,
    row.caseId,
    `sample-${row.sample}`,
  );
  ensurePrivateDirectory(outputRoot, "output-root-invalid");
  ensurePrivateDirectory(
    path.join(outputRoot, row.caseId),
    "artifact-directory-invalid",
  );
  ensurePrivateDirectory(artifactDirectory, "artifact-directory-invalid");
  const artifact = {
    schemaVersion: 1,
    benchmarkId: "downstream-code-quality",
    caseId: row.caseId,
    taskType: row.taskType,
    conditionId: row.mode,
    sampleIndex: row.sample,
    providerLabel: expectedProvider,
    baselineOid: row.baselineOid,
    runId: row.runId,
    contractSha256: row.contractSha256,
    workspaceManifestSha256: row.workspaceManifestSha256,
    runtimeManifestSha256,
    matrixHash: row.matrixHash,
    fixtureDigest: row.fixtureDigest,
    inputHash: row.inputHash,
    compositionHash: row.compositionHash,
    promotionEligible: false,
    scoringMode: "trusted-source-rebuild",
    pass: report.pass,
    outcomeClass: report.outcomeClass,
    trustedFixtureSha256: report.trustedFixtureSha256,
    verifierCompositionSha256: report.verifierCompositionSha256,
    changeEvidence: report.changeEvidence,
    gates: report.gates,
    verifier: report.verifier,
  };
  const destination = path.join(artifactDirectory, `${row.mode}.json`);
  const existing = fs.lstatSync(destination, { throwIfNoEntry: false });
  if (existing) {
    throw classifiedError("provenance-failure", "artifact-duplicate");
  }
  const temporary = path.join(
    artifactDirectory,
    `.${row.mode}.${process.pid}.${crypto.randomBytes(8).toString("hex")}.tmp`,
  );
  try {
    fs.writeFileSync(temporary, `${JSON.stringify(artifact, null, 2)}\n`, {
      flag: "wx",
      mode: 0o600,
    });
    try {
      fs.linkSync(temporary, destination);
    } catch (error) {
      if (error?.code === "EEXIST") {
        throw classifiedError("provenance-failure", "artifact-duplicate");
      }
      throw error;
    }
  } catch (error) {
    try {
      fs.rmSync(temporary, { force: true });
    } catch {
      // The classified write failure remains the primary operational result.
    }
    if (error?.message === "provenance-failure:artifact-duplicate") {
      throw error;
    }
    throw classifiedError("operational-failure", "artifact-write-failed");
  } finally {
    try {
      fs.unlinkSync(temporary);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw classifiedError("operational-failure", "artifact-write-failed");
      }
    }
  }
  return artifact;
}

module.exports = function assertExpenseReport(_untrustedOutput, context = {}) {
  const { expectedProvider, outputRoot, row, runtimeManifestSha256 } =
    findWorkspace(context);
  const report = runScorer(row);
  const artifact = writeArtifact({
    expectedProvider,
    outputRoot,
    report,
    row,
    runtimeManifestSha256,
  });
  const reasons = {
    pass: "Trusted source and deterministic gates passed",
    "candidate-failure": "Candidate failed one or more deterministic gates",
    "safety-failure": "Candidate source tree failed the safety gate",
  };
  return {
    pass: artifact.pass,
    score: artifact.pass ? 1 : 0,
    reason: reasons[artifact.outcomeClass],
  };
};
