const { spawnSync } = require("node:child_process");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { inputHashFor } = require("./benchmark-inputs.cjs");
const { loadWorkspaceManifest } = require("./manifest.cjs");

const runtimeEvidenceScript = path.resolve(
  __dirname,
  "../../../scripts/evals/code-quality-runtime-evidence.mjs",
);

const runtimeMarkerName = ".ai-plugins-code-quality-runtime-root";
const runtimeMarkerContents =
  "ai-plugins downstream code-quality runtime root\n";
const evalHomeMarkerName = ".ai-plugins-eval-home";
const evalHomeMarkerContents = "ai-plugins Codex eval home\n";
const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const skillPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*:[a-z0-9]+(?:-[a-z0-9]+)*$/;
const sha256Pattern = /^[0-9a-f]{64}$/;
const runtimeKeys = [
  "benchmarkId",
  "contractSha256",
  "matrixHash",
  "rows",
  "runId",
  "runtimeRoot",
  "schemaVersion",
  "workspaceManifest",
  "workspaceManifestSha256",
];
const runtimeRowKeys = [
  "availableSkills",
  "baselineOid",
  "caseId",
  "codexHome",
  "codexTmp",
  "compositionHash",
  "contractSha256",
  "fixtureDigest",
  "inputHash",
  "matrixHash",
  "mode",
  "runId",
  "sample",
  "workspace",
  "workspaceManifestSha256",
];

class RuntimeManifestError extends Error {
  constructor(category, code, detail = code.replaceAll("-", " ")) {
    super(detail);
    this.name = "RuntimeManifestError";
    this.category = category;
    this.code = code;
  }
}

function runtimeError(category, code, detail) {
  throw new RuntimeManifestError(category, code, detail);
}

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonicalize(value[key])]),
    );
  }
  return value;
}

function hashCanonical(value) {
  return sha256(JSON.stringify(canonicalize(value)));
}

function assertPlainObject(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
}

function assertExactKeys(value, keys, label) {
  if (
    JSON.stringify(Object.keys(value).sort()) !==
    JSON.stringify([...keys].sort())
  ) {
    throw new Error(`${label} has unexpected fields`);
  }
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

function assertPrivateDirectory(directory, label) {
  const stat = fs.lstatSync(directory, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isDirectory() ||
    stat.isSymbolicLink() ||
    (stat.mode & 0o077) !== 0 ||
    fs.realpathSync(directory) !== directory
  ) {
    throw new Error(`${label} must be a canonical private directory`);
  }
}

function assertPrivateFile(file, label, contents) {
  const stat = fs.lstatSync(file, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    (stat.mode & 0o077) !== 0 ||
    fs.realpathSync(file) !== file
  ) {
    throw new Error(`${label} must be a canonical private file`);
  }
  if (contents !== undefined && fs.readFileSync(file, "utf8") !== contents) {
    throw new Error(`${label} contents are invalid`);
  }
  return stat;
}

function loadRuntimeFile() {
  const configured = process.env.CODE_QUALITY_RUNTIME_MANIFEST;
  if (!configured || !path.isAbsolute(configured)) {
    runtimeError(
      "operational",
      "runtime-manifest-unavailable",
      "CODE_QUALITY_RUNTIME_MANIFEST must be an absolute manifest path",
    );
  }
  if (
    path.resolve(configured) !== configured ||
    path.basename(configured) !== "manifest.json"
  ) {
    runtimeError(
      "operational",
      "runtime-manifest-unavailable",
      "runtime manifest path is not canonical",
    );
  }
  let configuredStat;
  try {
    configuredStat = fs.lstatSync(configured, { throwIfNoEntry: false });
  } catch {
    runtimeError("operational", "runtime-manifest-unavailable");
  }
  if (!configuredStat) {
    runtimeError("operational", "runtime-manifest-unavailable");
  }
  const stat = assertPrivateFile(configured, "runtime manifest");
  if (stat.size < 2 || stat.size > 1024 * 1024) {
    throw new Error("runtime manifest size is invalid");
  }
  const runtimeRoot = path.dirname(configured);
  assertPrivateDirectory(runtimeRoot, "runtime root");
  const temporaryRoot = fs.realpathSync(os.tmpdir());
  if (!isStrictDescendant(temporaryRoot, runtimeRoot)) {
    throw new Error(`runtime root must be below ${temporaryRoot}`);
  }
  assertPrivateFile(
    path.join(runtimeRoot, runtimeMarkerName),
    "runtime root ownership marker",
    runtimeMarkerContents,
  );
  const bytes = fs.readFileSync(configured);
  let manifest;
  try {
    manifest = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new Error("runtime manifest is not valid JSON");
  }
  return {
    manifest,
    runtimeManifestPath: configured,
    runtimeManifestSha256: sha256(bytes),
    runtimeRoot,
  };
}

function loadRuntimeEvidence(codexHome, mode, phase) {
  const result = spawnSync(
    process.execPath,
    [
      runtimeEvidenceScript,
      "--codex-home",
      codexHome,
      "--mode",
      mode,
      "--phase",
      phase,
    ],
    {
      cwd: __dirname,
      encoding: "utf8",
      env: { LANG: "C.UTF-8", LC_ALL: "C.UTF-8" },
      killSignal: "SIGKILL",
      maxBuffer: 64 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 30_000,
    },
  );
  if (result.error) {
    runtimeError("operational", "runtime-evidence-unavailable");
  }
  if (result.status !== 0) {
    const match = String(result.stderr || "")
      .trim()
      .match(
        /^code-quality-runtime-evidence:(operational|provenance):([a-z0-9-]+)$/,
      );
    if (!match) runtimeError("operational", "runtime-evidence-unavailable");
    runtimeError(match[1], match[2]);
  }
  let evidence;
  try {
    evidence = JSON.parse(result.stdout);
  } catch {
    runtimeError("operational", "runtime-evidence-invalid");
  }
  if (
    !evidence ||
    typeof evidence !== "object" ||
    Array.isArray(evidence) ||
    JSON.stringify(Object.keys(evidence).sort()) !==
      JSON.stringify(["availableSkills", "compositionHash"]) ||
    !validateSkills(evidence.availableSkills) ||
    !sha256Pattern.test(evidence.compositionHash)
  ) {
    runtimeError("operational", "runtime-evidence-invalid");
  }
  return evidence;
}

function rowKey(row) {
  return `${row.caseId}\0${row.sample}\0${row.mode}`;
}

function validateSkills(skills) {
  return (
    Array.isArray(skills) &&
    skills.length <= 1024 &&
    skills.every(
      (skill) => typeof skill === "string" && skillPattern.test(skill),
    ) &&
    JSON.stringify(skills) === JSON.stringify([...new Set(skills)].sort())
  );
}

function validateRuntimeRow(
  entry,
  {
    manifest,
    runtimeRoot,
    phase,
    seenRuntimePaths,
    workspaceManifestSha256,
    workspaceRow,
  },
) {
  assertPlainObject(entry, "runtime row");
  assertExactKeys(entry, runtimeRowKeys, "runtime row");
  if (
    entry.caseId !== workspaceRow.caseId ||
    entry.sample !== workspaceRow.sample ||
    entry.mode !== workspaceRow.mode ||
    entry.workspace !== workspaceRow.workspace ||
    entry.baselineOid !== workspaceRow.baselineOid ||
    entry.fixtureDigest !== workspaceRow.fixtureDigest
  ) {
    throw new Error("runtime row workspace binding does not match");
  }
  if (
    entry.runId !== manifest.runId ||
    entry.contractSha256 !== manifest.contractSha256 ||
    entry.workspaceManifestSha256 !== workspaceManifestSha256 ||
    entry.matrixHash !== manifest.matrixHash
  ) {
    throw new Error("runtime row run identity does not match");
  }
  if (
    !sha256Pattern.test(entry.inputHash) ||
    !sha256Pattern.test(entry.compositionHash) ||
    !validateSkills(entry.availableSkills)
  ) {
    throw new Error("runtime row evidence is invalid");
  }
  const relativeRow = path.join(
    entry.caseId,
    `sample-${entry.sample}`,
    entry.mode,
  );
  const expectedRowRoot = path.join(runtimeRoot, relativeRow);
  const expectedCodexHome = path.join(expectedRowRoot, "codex-home");
  const expectedCodexTmp = path.join(expectedRowRoot, "tmp");
  if (
    entry.codexHome !== expectedCodexHome ||
    entry.codexTmp !== expectedCodexTmp
  ) {
    throw new Error("runtime row paths do not match their binding");
  }
  for (const [candidate, label] of [
    [expectedRowRoot, "runtime row root"],
    [expectedCodexHome, "runtime Codex home"],
    [expectedCodexTmp, "runtime tmp directory"],
  ]) {
    assertPrivateDirectory(candidate, label);
    if (seenRuntimePaths.has(candidate)) {
      throw new Error("runtime row reuses a private path");
    }
    seenRuntimePaths.add(candidate);
    if (pathsOverlap(candidate, workspaceRow.workspace)) {
      throw new Error("runtime row path overlaps its workspace");
    }
  }
  assertPrivateFile(
    path.join(expectedCodexHome, evalHomeMarkerName),
    "runtime Codex home marker",
    evalHomeMarkerContents,
  );
  const evidence = loadRuntimeEvidence(expectedCodexHome, entry.mode, phase);
  if (
    evidence.compositionHash !== entry.compositionHash ||
    JSON.stringify(evidence.availableSkills) !==
      JSON.stringify(entry.availableSkills)
  ) {
    runtimeError(
      "provenance",
      "runtime-composition-mismatch",
      "runtime composition does not match recorded evidence",
    );
  }
  if (entry.inputHash !== inputHashFor(workspaceRow, manifest.benchmarkId)) {
    runtimeError(
      "provenance",
      "runtime-input-mismatch",
      "runtime input does not match the rendered benchmark prompt",
    );
  }
  return { ...workspaceRow, ...entry };
}

function assertWorkspaceManifestAvailable() {
  const configured = process.env.CODE_QUALITY_WORKSPACE_MANIFEST;
  if (!configured || !path.isAbsolute(configured)) {
    runtimeError(
      "operational",
      "workspace-manifest-unavailable",
      "CODE_QUALITY_WORKSPACE_MANIFEST must identify an available absolute manifest path",
    );
  }
  let stat;
  try {
    stat = fs.lstatSync(configured, { throwIfNoEntry: false });
  } catch {
    runtimeError("operational", "workspace-manifest-unavailable");
  }
  if (!stat) {
    runtimeError("operational", "workspace-manifest-unavailable");
  }
  if (stat.isFile() && !stat.isSymbolicLink()) {
    let descriptor;
    try {
      descriptor = fs.openSync(
        configured,
        fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW,
      );
    } catch {
      runtimeError("operational", "workspace-manifest-unavailable");
    } finally {
      if (descriptor !== undefined) fs.closeSync(descriptor);
    }
  }
}

function loadWorkspaceState(inspectGit) {
  assertWorkspaceManifestAvailable();
  try {
    return loadWorkspaceManifest({ inspectGit });
  } catch (error) {
    let stillAvailable = true;
    try {
      stillAvailable = Boolean(
        fs.lstatSync(process.env.CODE_QUALITY_WORKSPACE_MANIFEST, {
          throwIfNoEntry: false,
        }),
      );
    } catch {
      stillAvailable = false;
    }
    if (!stillAvailable) {
      runtimeError("operational", "workspace-manifest-unavailable");
    }
    throw error;
  }
}

function loadRuntimeManifestUnchecked({
  inspectGit = true,
  phase = "post-turn",
  workspaceState,
} = {}) {
  if (!["pre-turn", "post-turn"].includes(phase)) {
    runtimeError("operational", "runtime-phase-invalid");
  }
  const state = workspaceState || loadWorkspaceState(inspectGit);
  const runtimeFile = loadRuntimeFile();
  const { manifest, runtimeRoot } = runtimeFile;
  assertPlainObject(manifest, "runtime manifest");
  assertExactKeys(manifest, runtimeKeys, "runtime manifest");
  if (
    manifest.schemaVersion !== 1 ||
    manifest.benchmarkId !== state.manifest.benchmarkId ||
    manifest.runId !== state.manifest.runId ||
    manifest.contractSha256 !== state.manifest.contractSha256 ||
    manifest.runtimeRoot !== runtimeRoot ||
    manifest.workspaceManifest !== state.manifestPath
  ) {
    throw new Error("runtime manifest header does not match the workspace run");
  }
  if (pathsOverlap(runtimeRoot, state.workRoot)) {
    throw new Error("runtime root overlaps the workspace root");
  }
  let workspaceManifestBytes;
  try {
    workspaceManifestBytes = fs.readFileSync(state.manifestPath);
  } catch {
    runtimeError("operational", "workspace-manifest-unavailable");
  }
  const workspaceManifestSha256 = sha256(workspaceManifestBytes);
  if (manifest.workspaceManifestSha256 !== workspaceManifestSha256) {
    throw new Error("runtime manifest workspace digest does not match");
  }
  if (!Array.isArray(manifest.rows)) {
    throw new Error("runtime manifest rows must be an array");
  }
  const matrixHash = hashCanonical({
    contractSha256: manifest.contractSha256,
    rows: state.rows.map((row) => ({
      baselineOid: row.baselineOid,
      caseId: row.caseId,
      fixtureDigest: row.fixtureDigest,
      mode: row.mode,
      sample: row.sample,
      workspace: row.workspace,
    })),
    schemaVersion: 1,
    workspaceManifestSha256,
  });
  if (manifest.matrixHash !== matrixHash) {
    throw new Error("runtime manifest matrix hash does not match");
  }
  if (manifest.rows.length !== state.rows.length) {
    throw new Error("runtime manifest does not contain the exact matrix");
  }
  const workspaceRows = new Map(state.rows.map((row) => [rowKey(row), row]));
  const seenRows = new Set();
  const seenRuntimePaths = new Set();
  const rows = manifest.rows.map((entry) => {
    if (
      !entry ||
      !identifierPattern.test(entry.caseId || "") ||
      !identifierPattern.test(entry.mode || "") ||
      !Number.isInteger(entry.sample)
    ) {
      throw new Error("runtime row identity is invalid");
    }
    const key = rowKey(entry);
    const workspaceRow = workspaceRows.get(key);
    if (!workspaceRow || seenRows.has(key)) {
      throw new Error("runtime manifest contains an invalid row binding");
    }
    seenRows.add(key);
    return validateRuntimeRow(entry, {
      manifest,
      phase,
      runtimeRoot,
      seenRuntimePaths,
      workspaceManifestSha256,
      workspaceRow,
    });
  });
  if (seenRows.size !== workspaceRows.size) {
    throw new Error("runtime manifest does not contain the exact matrix");
  }
  return { ...runtimeFile, manifest, rows, workspaceState: state };
}

function loadRuntimeManifest(options) {
  try {
    return loadRuntimeManifestUnchecked(options);
  } catch (error) {
    if (error instanceof RuntimeManifestError) throw error;
    throw new RuntimeManifestError(
      "provenance",
      "runtime-manifest-invalid",
      error instanceof Error ? error.message : "runtime manifest is invalid",
    );
  }
}

module.exports = { loadRuntimeManifest, RuntimeManifestError };
