const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const benchmarkDirectory = __dirname;
const contract = JSON.parse(
  fs.readFileSync(path.join(benchmarkDirectory, "benchmark.json"), "utf8"),
);
const contractSha256 = require("node:crypto")
  .createHash("sha256")
  .update(fs.readFileSync(path.join(benchmarkDirectory, "benchmark.json")))
  .digest("hex");
const rootMarkerName = ".ai-plugins-code-quality-work-root";
const rootMarkerContents = "ai-plugins downstream code-quality work root\n";
const workspaceMarkerName = ".git/.ai-plugins-code-quality-workspace";
const workspaceMarkerContents =
  "ai-plugins downstream code-quality workspace\n";
const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const oidPattern = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
const sha256Pattern = /^[0-9a-f]{64}$/;

function assertPlainObject(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
}

function assertIdentifier(value, label) {
  if (typeof value !== "string" || !identifierPattern.test(value)) {
    throw new Error(`invalid ${label}`);
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

function assertRegularFileWithContents(file, contents, label) {
  const stat = fs.lstatSync(file, { throwIfNoEntry: false });
  if (!stat || !stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(`${label} must be a regular file`);
  }
  if (fs.readFileSync(file, "utf8") !== contents) {
    throw new Error(`${label} contents are invalid`);
  }
}

function git(workspace, args) {
  return execFileSync("git", args, {
    cwd: workspace,
    encoding: "utf8",
    env: {
      PATH: process.env.PATH,
      HOME: workspace,
      LANG: "C.UTF-8",
      LC_ALL: "C.UTF-8",
      GIT_CONFIG_GLOBAL: "/dev/null",
      GIT_CONFIG_NOSYSTEM: "1",
    },
    killSignal: "SIGKILL",
    maxBuffer: 256 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 5_000,
  }).trim();
}

function loadManifestFile() {
  const configuredPath = process.env.CODE_QUALITY_WORKSPACE_MANIFEST;
  if (!configuredPath || !path.isAbsolute(configuredPath)) {
    throw new Error(
      "CODE_QUALITY_WORKSPACE_MANIFEST must be an absolute manifest path",
    );
  }
  const manifestStat = fs.lstatSync(configuredPath, { throwIfNoEntry: false });
  if (
    !manifestStat ||
    !manifestStat.isFile() ||
    manifestStat.isSymbolicLink()
  ) {
    throw new Error("workspace manifest must be a regular file");
  }
  const manifestPath = fs.realpathSync(configuredPath);
  if (
    manifestPath !== path.resolve(configuredPath) ||
    path.basename(manifestPath) !== "manifest.json"
  ) {
    throw new Error("workspace manifest path is not canonical");
  }

  const workRoot = fs.realpathSync(path.dirname(manifestPath));
  const temporaryRoot = fs.realpathSync(os.tmpdir());
  if (!isStrictDescendant(temporaryRoot, workRoot)) {
    throw new Error(`workspace manifest root must be below ${temporaryRoot}`);
  }
  assertRegularFileWithContents(
    path.join(workRoot, rootMarkerName),
    rootMarkerContents,
    "workspace root ownership marker",
  );

  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  } catch {
    throw new Error("workspace manifest is not valid JSON");
  }
  return { manifest, manifestPath, workRoot };
}

function validateManifestHeader(manifest) {
  assertPlainObject(manifest, "workspace manifest");
  if (manifest.schemaVersion !== 1) {
    throw new Error("workspace manifest schemaVersion must be 1");
  }
  if (manifest.benchmarkId !== contract.id) {
    throw new Error("workspace manifest benchmark id does not match");
  }
  if (!sha256Pattern.test(manifest.runId)) {
    throw new Error("workspace manifest runId must be 64 lowercase hex");
  }
  if (manifest.contractSha256 !== contractSha256) {
    throw new Error("workspace manifest contract digest does not match");
  }
  if (
    !Number.isInteger(manifest.sampleCount) ||
    manifest.sampleCount < 1 ||
    manifest.sampleCount > 10
  ) {
    throw new Error("workspace manifest sampleCount must be from 1 through 10");
  }
  if (
    process.env.EVAL_SAMPLES &&
    process.env.EVAL_SAMPLES !== String(manifest.sampleCount)
  ) {
    throw new Error("EVAL_SAMPLES does not match the workspace manifest");
  }
  if (!Array.isArray(manifest.workspaces) || manifest.workspaces.length === 0) {
    throw new Error("workspace manifest must contain workspaces");
  }
}

function selectedCases(manifest) {
  const configuredFilter = process.env.EVAL_CASE_FILTER;
  const manifestCaseIds = [
    ...new Set(manifest.workspaces.map((workspace) => workspace?.caseId)),
  ];
  const selected = configuredFilter
    ? contract.cases.filter((testCase) => testCase.id === configuredFilter)
    : contract.cases.filter((testCase) =>
        manifestCaseIds.includes(testCase.id),
      );
  if (selected.length === 0) {
    throw new Error("workspace manifest selects no configured benchmark case");
  }
  if (
    configuredFilter &&
    manifestCaseIds.some((caseId) => caseId !== configuredFilter)
  ) {
    throw new Error(
      "workspace manifest contains a case outside EVAL_CASE_FILTER",
    );
  }
  return selected;
}

function validateWorkspace(
  entry,
  { caseById, inspectGit, requireBaselineHead, requireClean, workRoot },
) {
  assertPlainObject(entry, "benchmark workspace");
  assertIdentifier(entry.caseId, "workspace case id");
  assertIdentifier(entry.taskType, "workspace task type");
  assertIdentifier(entry.mode, "workspace condition id");
  const caseConfig = caseById.get(entry.caseId);
  if (!caseConfig || caseConfig.taskType !== entry.taskType) {
    throw new Error(`workspace case metadata does not match: ${entry.caseId}`);
  }
  if (!contract.conditions.some((condition) => condition.id === entry.mode)) {
    throw new Error(`unknown workspace condition: ${entry.mode}`);
  }
  if (
    !Number.isInteger(entry.sample) ||
    entry.sample < 1 ||
    entry.sample > 10
  ) {
    throw new Error("workspace sample must be from 1 through 10");
  }
  if (
    typeof entry.workspace !== "string" ||
    !path.isAbsolute(entry.workspace)
  ) {
    throw new Error("workspace path must be absolute");
  }
  const expectedWorkspace = path.join(
    workRoot,
    entry.caseId,
    `sample-${entry.sample}`,
    entry.mode,
  );
  if (path.resolve(entry.workspace) !== expectedWorkspace) {
    throw new Error("workspace path does not match its manifest binding");
  }
  const workspaceStat = fs.lstatSync(expectedWorkspace, {
    throwIfNoEntry: false,
  });
  if (
    !workspaceStat ||
    !workspaceStat.isDirectory() ||
    workspaceStat.isSymbolicLink()
  ) {
    throw new Error("workspace path must be a real directory");
  }
  if (fs.realpathSync(expectedWorkspace) !== expectedWorkspace) {
    throw new Error("workspace path is not canonical");
  }
  assertRegularFileWithContents(
    path.join(expectedWorkspace, workspaceMarkerName),
    workspaceMarkerContents,
    "prepared benchmark workspace marker",
  );
  if (
    typeof entry.baselineOid !== "string" ||
    !oidPattern.test(entry.baselineOid)
  ) {
    throw new Error("workspace baseline OID is invalid");
  }
  if (
    typeof entry.fixtureDigest !== "string" ||
    !sha256Pattern.test(entry.fixtureDigest)
  ) {
    throw new Error("workspace fixture digest is invalid");
  }
  if (
    inspectGit &&
    requireBaselineHead &&
    git(expectedWorkspace, ["rev-parse", "HEAD"]) !== entry.baselineOid
  ) {
    throw new Error(
      `baseline OID does not match workspace HEAD: ${entry.caseId}`,
    );
  }
  if (
    inspectGit &&
    requireClean &&
    git(expectedWorkspace, ["status", "--porcelain"]) !== ""
  ) {
    throw new Error(
      `prepared benchmark workspace is not clean: ${entry.caseId}`,
    );
  }
  if (inspectGit && git(expectedWorkspace, ["remote"]) !== "") {
    throw new Error(`benchmark workspace has a Git remote: ${entry.caseId}`);
  }
  return { ...entry, workspace: expectedWorkspace };
}

function loadWorkspaceManifest({
  inspectGit = true,
  requireBaselineHead = false,
  requireClean = false,
} = {}) {
  const { manifest, manifestPath, workRoot } = loadManifestFile();
  validateManifestHeader(manifest);
  const cases = selectedCases(manifest);
  const caseById = new Map(cases.map((testCase) => [testCase.id, testCase]));
  const seen = new Set();
  const rows = manifest.workspaces.map((entry) => {
    const row = validateWorkspace(entry, {
      caseById,
      inspectGit,
      requireBaselineHead,
      requireClean,
      workRoot,
    });
    if (row.sample > manifest.sampleCount) {
      throw new Error("workspace sample exceeds manifest sampleCount");
    }
    const key = `${row.caseId}\0${row.sample}\0${row.mode}`;
    if (seen.has(key)) {
      throw new Error(`duplicate benchmark workspace binding: ${row.caseId}`);
    }
    seen.add(key);
    return row;
  });

  const expectedKeys = [];
  for (const testCase of cases) {
    for (let sample = 1; sample <= manifest.sampleCount; sample += 1) {
      for (const condition of contract.conditions) {
        expectedKeys.push(`${testCase.id}\0${sample}\0${condition.id}`);
      }
    }
  }
  if (
    rows.length !== expectedKeys.length ||
    expectedKeys.some((key) => !seen.has(key))
  ) {
    throw new Error(
      "workspace manifest does not contain the exact benchmark matrix",
    );
  }

  return { contract, manifest, manifestPath, rows, workRoot };
}

function providerLabelFor(mode) {
  return `openai-codex-sdk-${mode}`;
}

module.exports = {
  loadWorkspaceManifest,
  providerLabelFor,
};
