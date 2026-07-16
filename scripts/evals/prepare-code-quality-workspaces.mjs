#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { snapshotRegularTree } from "./code-quality-tree-hash.mjs";
import { loadBenchmarkContract } from "./validate-code-quality-contract.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const benchmarkDir = path.join(
  root,
  "evals/benchmarks/downstream-code-quality",
);
const contractPath = path.join(benchmarkDir, "benchmark.json");
const contractBytes = fs.readFileSync(contractPath);
const contract = loadBenchmarkContract(contractPath);
const contractSha256 = crypto
  .createHash("sha256")
  .update(contractBytes)
  .digest("hex");
const rootMarker = ".ai-plugins-code-quality-work-root";
const rootMarkerContents = "ai-plugins downstream code-quality work root\n";
const workspaceMarker = "ai-plugins downstream code-quality workspace\n";

class WorkRootLockError extends Error {
  constructor(workRoot) {
    super(`workspace preparation already active for root: ${workRoot}`);
    this.exitCode = 75;
  }
}

function usage() {
  console.error(
    "usage: prepare-code-quality-workspaces.mjs <work-root> --case CASE_ID [--samples 1-10]",
  );
}

function parseArgs(argv) {
  const args = {
    workRoot: argv[0],
    caseId: null,
    samples: contract.sampleCount,
  };
  for (let index = 1; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--case") {
      args.caseId = argv[++index];
    } else if (argument === "--samples") {
      args.samples = Number(argv[++index]);
    } else {
      throw new Error(`unknown argument: ${argument}`);
    }
  }
  if (!args.workRoot) {
    usage();
    process.exit(2);
  }
  if (!args.caseId) {
    throw new Error(
      "--case is required until every declared fixture is available",
    );
  }
  if (
    !Number.isInteger(args.samples) ||
    args.samples < 1 ||
    args.samples > 10
  ) {
    throw new Error("--samples must be an integer from 1 through 10");
  }
  return args;
}

function canonicalProspectivePath(value) {
  const absolute = path.resolve(value);
  const missing = [];
  let existing = absolute;
  while (!fs.existsSync(existing)) {
    const parent = path.dirname(existing);
    if (parent === existing) {
      throw new Error(`cannot resolve path: ${value}`);
    }
    missing.unshift(path.basename(existing));
    existing = parent;
  }
  return path.join(fs.realpathSync(existing), ...missing);
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

function assertNoSymlinkComponents(allowedRoot, target) {
  const relative = path.relative(allowedRoot, target);
  let current = allowedRoot;
  for (const segment of relative.split(path.sep).filter(Boolean)) {
    current = path.join(current, segment);
    if (fs.existsSync(current) && fs.lstatSync(current).isSymbolicLink()) {
      throw new Error(`workspace path contains a symlink: ${current}`);
    }
  }
}

function assertReplaceableWorkRoot(workRoot) {
  if (!fs.existsSync(workRoot)) return;
  if (!fs.statSync(workRoot).isDirectory()) {
    throw new Error(`workspace root is not a directory: ${workRoot}`);
  }
  const entries = fs.readdirSync(workRoot);
  if (entries.length === 0) return;
  const marker = path.join(workRoot, rootMarker);
  const markerStat = fs.lstatSync(marker, { throwIfNoEntry: false });
  if (markerStat && !markerStat.isFile()) {
    throw new Error(`ownership marker must be a regular file: ${marker}`);
  }
  if (!markerStat || fs.readFileSync(marker, "utf8") !== rootMarkerContents) {
    throw new Error(`refusing to replace unowned workspace root: ${workRoot}`);
  }
}

function acquireWorkRootLock(workRoot) {
  const userId =
    typeof process.getuid === "function"
      ? String(process.getuid())
      : os.userInfo().username.replaceAll(/[^A-Za-z0-9_.-]/g, "_");
  const lockRoot = path.join(
    os.tmpdir(),
    `ai-plugins-code-quality-locks-${userId}`,
  );
  fs.mkdirSync(lockRoot, { mode: 0o700, recursive: true });
  const lockRootStat = fs.lstatSync(lockRoot);
  if (!lockRootStat.isDirectory() || lockRootStat.isSymbolicLink()) {
    throw new Error(`workspace lock root is not a real directory: ${lockRoot}`);
  }

  const lockHash = crypto.createHash("sha256").update(workRoot).digest("hex");
  const lockFile = path.join(lockRoot, `${lockHash}.lock`);
  const lockStat = fs.lstatSync(lockFile, { throwIfNoEntry: false });
  if (lockStat && !lockStat.isFile()) {
    throw new Error(`workspace lock must be a regular file: ${lockFile}`);
  }
  const lockFd = fs.openSync(lockFile, "a", 0o600);
  const result = spawnSync("flock", ["--nonblock", "3"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe", lockFd],
  });
  if (result.status !== 0) {
    fs.closeSync(lockFd);
    if (result.status === 1) {
      throw new WorkRootLockError(workRoot);
    }
    throw new Error(
      `failed to acquire workspace lock: ${result.error?.message || result.stderr || result.stdout}`,
    );
  }
  return lockFd;
}

function git(workspace, args, options = {}) {
  const environment = {
    PATH: process.env.PATH,
    HOME: workspace,
    LANG: "C.UTF-8",
    LC_ALL: "C.UTF-8",
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_AUTHOR_NAME: "Developer",
    GIT_AUTHOR_EMAIL: "developer@example.invalid",
    GIT_COMMITTER_NAME: "Developer",
    GIT_COMMITTER_EMAIL: "developer@example.invalid",
    GIT_AUTHOR_DATE: "2000-01-01T00:00:00Z",
    GIT_COMMITTER_DATE: "2000-01-01T00:00:00Z",
  };
  const result = spawnSync("git", args, {
    cwd: workspace,
    env: environment,
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(
      `git ${args.join(" ")} failed in prepared workspace: ${result.stderr || result.stdout}`,
    );
  }
  return result.stdout.trim();
}

function assertFixtureHasNoSymlinks(entry) {
  const stat = fs.lstatSync(entry);
  if (stat.isSymbolicLink()) {
    throw new Error(`fixture contains a symlink: ${entry}`);
  }
  if (stat.isDirectory()) {
    for (const child of fs.readdirSync(entry)) {
      assertFixtureHasNoSymlinks(path.join(entry, child));
    }
  }
}

function resolveFixture(caseConfig) {
  const fixturesRoot = fs.realpathSync(path.join(benchmarkDir, "fixtures"));
  const fixture = canonicalProspectivePath(
    path.join(fixturesRoot, caseConfig.fixture),
  );
  if (!isStrictDescendant(fixturesRoot, fixture)) {
    throw new Error(`fixture escapes benchmark fixture root: ${fixture}`);
  }
  if (!fs.existsSync(fixture)) {
    throw new Error(`missing benchmark fixture: ${caseConfig.fixture}`);
  }
  if (!fs.statSync(fixture).isDirectory()) {
    throw new Error(
      `benchmark fixture is not a directory: ${caseConfig.fixture}`,
    );
  }
  assertFixtureHasNoSymlinks(fixture);
  return fixture;
}

function prepareWorkspace({
  caseConfig,
  fixture,
  fixtureDigest,
  mode,
  sample,
  workRoot,
}) {
  const workspace = path.join(
    workRoot,
    caseConfig.id,
    `sample-${sample}`,
    mode,
  );
  if (!isStrictDescendant(workRoot, workspace)) {
    throw new Error(`workspace escapes disposable root: ${workspace}`);
  }
  fs.mkdirSync(path.dirname(workspace), { recursive: true });
  fs.cpSync(fixture, workspace, {
    recursive: true,
    errorOnExist: true,
    force: false,
  });
  git(workspace, ["init", "--quiet", "--initial-branch=main"]);
  git(workspace, ["add", "--all"]);
  git(workspace, ["commit", "--quiet", "-m", "Initial project state"]);
  fs.writeFileSync(
    path.join(workspace, ".git/.ai-plugins-code-quality-workspace"),
    workspaceMarker,
  );

  const baselineOid = git(workspace, ["rev-parse", "HEAD"]);
  if (git(workspace, ["remote"]) !== "") {
    throw new Error(
      `prepared workspace unexpectedly has a remote: ${workspace}`,
    );
  }
  if (git(workspace, ["status", "--porcelain"]) !== "") {
    throw new Error(`prepared workspace is not clean: ${workspace}`);
  }

  return {
    caseId: caseConfig.id,
    taskType: caseConfig.taskType,
    mode,
    sample,
    workspace,
    baselineOid,
    fixtureDigest,
  };
}

let workRootLockFd;
try {
  const args = parseArgs(process.argv.slice(2));
  const allowedRoot = fs.realpathSync(os.tmpdir());
  const workRoot = canonicalProspectivePath(args.workRoot);
  if (!isStrictDescendant(allowedRoot, workRoot)) {
    throw new Error(`workspace root must be below ${allowedRoot}: ${workRoot}`);
  }
  assertNoSymlinkComponents(allowedRoot, workRoot);
  workRootLockFd = acquireWorkRootLock(workRoot);
  assertReplaceableWorkRoot(workRoot);

  const selectedCases = contract.cases.filter(
    (entry) => entry.id === args.caseId,
  );
  if (selectedCases.length === 0) {
    throw new Error(`unknown benchmark case: ${args.caseId}`);
  }
  const fixturesByCase = new Map(
    selectedCases.map((caseConfig) => {
      const fixture = resolveFixture(caseConfig);
      return [
        caseConfig.id,
        { fixture, fixtureDigest: snapshotRegularTree(fixture).digest },
      ];
    }),
  );

  fs.rmSync(workRoot, { recursive: true, force: true });
  fs.mkdirSync(workRoot, { recursive: true });
  fs.writeFileSync(path.join(workRoot, rootMarker), rootMarkerContents);

  const workspaces = [];
  for (const caseConfig of selectedCases) {
    for (let sample = 1; sample <= args.samples; sample += 1) {
      for (const condition of contract.conditions) {
        const fixtureState = fixturesByCase.get(caseConfig.id);
        workspaces.push(
          prepareWorkspace({
            caseConfig,
            fixture: fixtureState.fixture,
            fixtureDigest: fixtureState.fixtureDigest,
            mode: condition.id,
            sample,
            workRoot,
          }),
        );
      }
    }
  }

  const manifest = {
    schemaVersion: 1,
    benchmarkId: contract.id,
    runId: crypto.randomBytes(32).toString("hex"),
    contractSha256,
    sampleCount: args.samples,
    workspaces,
  };
  fs.writeFileSync(
    path.join(workRoot, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  process.stdout.write(JSON.stringify(manifest));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = error instanceof WorkRootLockError ? error.exitCode : 2;
} finally {
  if (workRootLockFd !== undefined) {
    fs.closeSync(workRootLockFd);
  }
}
