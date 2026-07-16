#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import {
  canonicalJson,
  credentialNames,
  evalHomeMarkerContents,
  evalHomeMarkerName,
  executionSurfaceFromEnvironment,
  executionSurfaceName,
  hashCanonical,
  identifierPattern,
  runtimeConfigForPlugins,
  sanitizedMarketplaceManifest,
  sanitizedPluginManifest,
  selectMarketplacePlugins,
  sha256,
  sha256Pattern,
} from "./code-quality-runtime-contract.mjs";
import { measureRuntimeEvidence } from "./code-quality-runtime-evidence.mjs";
import {
  sha256TreeSnapshot,
  snapshotRegularTree,
  writeTreeSnapshot,
} from "./code-quality-tree-hash.mjs";
import { validateBenchmarkContract } from "./validate-code-quality-contract.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const benchmarkDirectory = path.join(
  root,
  "evals/benchmarks/downstream-code-quality",
);
const contractFile = path.join(benchmarkDirectory, "benchmark.json");
const marketplaceFile = path.join(root, ".agents/plugins/marketplace.json");
const runtimeMarkerName = ".ai-plugins-code-quality-runtime-root";
const runtimeMarkerContents =
  "ai-plugins downstream code-quality runtime root\n";
const systemMarkerName = ".codex-system-skills.marker";
const systemMarkerPattern = /^[0-9a-f]{16}\n$/;
const treeLimits = Object.freeze({
  directories: 4096,
  fileBytes: 1024 * 1024,
  files: 4096,
  pathBytes: 1024,
  totalBytes: 32 * 1024 * 1024,
});
const require = createRequire(import.meta.url);
const { loadWorkspaceManifest } = require(
  path.join(benchmarkDirectory, "manifest.cjs"),
);
const { inputHashFor } = require(
  path.join(benchmarkDirectory, "benchmark-inputs.cjs"),
);

function usage() {
  console.error(
    "usage: prepare-code-quality-runtime.mjs <absolute-workspace-manifest> <absolute-runtime-root>",
  );
}

function parseArguments(argv) {
  if (argv.length !== 2) {
    usage();
    throw new Error("workspace manifest and runtime root are required");
  }
  const [workspaceManifest, runtimeRoot] = argv;
  if (!path.isAbsolute(workspaceManifest)) {
    throw new Error("workspace manifest path must be absolute");
  }
  if (!path.isAbsolute(runtimeRoot)) {
    throw new Error("runtime root path must be absolute");
  }
  return { runtimeRoot, workspaceManifest };
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

function isSameOrAncestor(ancestor, descendant) {
  const relative = path.relative(ancestor, descendant);
  return (
    relative === "" ||
    (relative !== ".." &&
      !relative.startsWith(`..${path.sep}`) &&
      !path.isAbsolute(relative))
  );
}

function pathsOverlap(first, second) {
  return isSameOrAncestor(first, second) || isSameOrAncestor(second, first);
}

function assertNoSymlinkComponents(allowedRoot, target) {
  const relative = path.relative(allowedRoot, target);
  let current = allowedRoot;
  for (const segment of relative.split(path.sep).filter(Boolean)) {
    current = path.join(current, segment);
    const stat = fs.lstatSync(current, { throwIfNoEntry: false });
    if (stat?.isSymbolicLink()) {
      throw new Error(`runtime path contains a symlink: ${current}`);
    }
  }
}

function walkTree(entry, visitor) {
  const stat = fs.lstatSync(entry);
  visitor(entry, stat);
  if (stat.isDirectory()) {
    for (const child of fs.readdirSync(entry).sort()) {
      walkTree(path.join(entry, child), visitor);
    }
  }
}

function assertTreeHasNoSymlinksOrCredentials(entry) {
  walkTree(entry, (candidate, stat) => {
    if (stat.isSymbolicLink()) {
      throw new Error(`runtime tree contains a symlink: ${candidate}`);
    }
    if (!stat.isDirectory() && !stat.isFile()) {
      throw new Error(`runtime tree contains a special file: ${candidate}`);
    }
    if (stat.isFile() && credentialNames.has(path.basename(candidate))) {
      throw new Error(
        `runtime tree contains forbidden auth credentials: ${candidate}`,
      );
    }
  });
}

function assertRuntimeRootIsUnused(runtimeRoot) {
  const stat = fs.lstatSync(runtimeRoot, { throwIfNoEntry: false });
  if (!stat) return false;
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    throw new Error(`runtime root must be a real directory: ${runtimeRoot}`);
  }
  const entries = fs.readdirSync(runtimeRoot);
  if (entries.length === 0) return true;

  const marker = path.join(runtimeRoot, runtimeMarkerName);
  const markerStat = fs.lstatSync(marker, { throwIfNoEntry: false });
  if (
    !markerStat ||
    !markerStat.isFile() ||
    markerStat.isSymbolicLink() ||
    fs.readFileSync(marker, "utf8") !== runtimeMarkerContents
  ) {
    throw new Error(`refusing to replace unowned runtime root: ${runtimeRoot}`);
  }
  assertTreeHasNoSymlinksOrCredentials(runtimeRoot);
  throw new Error(`runtime artifacts already exist: ${runtimeRoot}`);
}

function protectedAuthSource() {
  return (
    process.env.CODEX_EVAL_AUTH_HOME ||
    process.env.CODEX_HOME ||
    path.join(os.homedir(), ".codex")
  );
}

function assertRuntimePathIsIsolated(runtimeRoot, workspaceState) {
  const temporaryRoot = fs.realpathSync(os.tmpdir());
  if (!isStrictDescendant(temporaryRoot, runtimeRoot)) {
    throw new Error(
      `runtime root must be below ${temporaryRoot}: ${runtimeRoot}`,
    );
  }
  assertNoSymlinkComponents(temporaryRoot, runtimeRoot);

  const protectedPaths = [
    ["repository", fs.realpathSync(root)],
    ["real home", canonicalProspectivePath(os.homedir())],
    ["auth source", canonicalProspectivePath(protectedAuthSource())],
    ["workspace root", workspaceState.workRoot],
    ["workspace manifest", workspaceState.manifestPath],
  ];
  for (const [label, protectedPath] of protectedPaths) {
    if (pathsOverlap(runtimeRoot, protectedPath)) {
      throw new Error(
        `runtime root overlaps protected ${label}: ${protectedPath}`,
      );
    }
  }
}

function assertRequestedRuntimePathHasNoSymlinks(runtimeRootArgument) {
  const requestedTemporaryRoot = path.resolve(os.tmpdir());
  const requestedRuntimeRoot = path.resolve(runtimeRootArgument);
  if (!isStrictDescendant(requestedTemporaryRoot, requestedRuntimeRoot)) {
    throw new Error(
      `runtime root must be below ${requestedTemporaryRoot}: ${requestedRuntimeRoot}`,
    );
  }
  assertNoSymlinkComponents(requestedTemporaryRoot, requestedRuntimeRoot);
}

function withWorkspaceManifestEnvironment(workspaceManifest, callback) {
  const names = [
    "CODE_QUALITY_WORKSPACE_MANIFEST",
    "EVAL_CASE_FILTER",
    "EVAL_SAMPLES",
  ];
  const previous = new Map(names.map((name) => [name, process.env[name]]));
  process.env.CODE_QUALITY_WORKSPACE_MANIFEST = workspaceManifest;
  delete process.env.EVAL_CASE_FILTER;
  delete process.env.EVAL_SAMPLES;
  try {
    return callback();
  } finally {
    for (const name of names) {
      const value = previous.get(name);
      if (value === undefined) delete process.env[name];
      else process.env[name] = value;
    }
  }
}

function loadValidatedInputs(workspaceManifestArgument) {
  const workspaceManifestStat = fs.lstatSync(workspaceManifestArgument, {
    throwIfNoEntry: false,
  });
  if (
    !workspaceManifestStat ||
    !workspaceManifestStat.isFile() ||
    workspaceManifestStat.isSymbolicLink() ||
    fs.realpathSync(workspaceManifestArgument) !==
      path.resolve(workspaceManifestArgument)
  ) {
    throw new Error("workspace manifest must be a canonical regular file");
  }
  const contractBytes = fs.readFileSync(contractFile);
  const contract = validateBenchmarkContract(
    JSON.parse(contractBytes.toString("utf8")),
  );
  const workspaceManifestBytesBefore = fs.readFileSync(
    workspaceManifestArgument,
  );
  const state = withWorkspaceManifestEnvironment(
    workspaceManifestArgument,
    () =>
      loadWorkspaceManifest({
        requireBaselineHead: true,
        requireClean: true,
      }),
  );

  if (canonicalJson(state.contract) !== canonicalJson(contract)) {
    throw new Error(
      "workspace validator contract does not match validated contract",
    );
  }
  const workspaceManifestBytesAfter = fs.readFileSync(state.manifestPath);
  if (!workspaceManifestBytesBefore.equals(workspaceManifestBytesAfter)) {
    throw new Error("workspace manifest changed while it was being validated");
  }
  const contractSha256 = sha256(contractBytes);
  if (
    typeof state.manifest.runId !== "string" ||
    !sha256Pattern.test(state.manifest.runId)
  ) {
    throw new Error(
      "workspace manifest runId must be a 256-bit lowercase hex value",
    );
  }
  if (state.manifest.contractSha256 !== contractSha256) {
    throw new Error(
      "workspace manifest contractSha256 does not match benchmark.json",
    );
  }
  return {
    contract,
    contractSha256,
    workspaceManifestSha256: sha256(workspaceManifestBytesAfter),
    ...state,
  };
}

function readJsonRegular(file, label) {
  const stat = fs.lstatSync(file, { throwIfNoEntry: false });
  if (!stat || !stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(`${label} must be a regular file`);
  }
  let bytes;
  try {
    bytes = fs.readFileSync(file);
  } catch {
    throw new Error(`${label} is unreadable`);
  }
  try {
    return { bytes, value: JSON.parse(bytes.toString("utf8")) };
  } catch {
    throw new Error(`${label} is not valid JSON`);
  }
}

function assertRegularDirectory(directory, label) {
  const stat = fs.lstatSync(directory, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isDirectory() ||
    stat.isSymbolicLink() ||
    fs.realpathSync(directory) !== path.resolve(directory)
  ) {
    throw new Error(`${label} must be a canonical real directory`);
  }
}

function slashPath(value) {
  return value.split(path.sep).join("/");
}

function hasForbiddenSegment(relative) {
  return slashPath(relative)
    .split("/")
    .some(
      (segment) => credentialNames.has(segment) || segment === ".plugin-eval",
    );
}

function directSkillNames(snapshot, label) {
  const directDirectories = snapshot.directories
    .map(slashPath)
    .filter((relative) => relative !== "" && !relative.includes("/"))
    .sort();
  const skills = directDirectories.filter((skill) =>
    snapshot.files.has(path.join(skill, "SKILL.md")),
  );
  for (const skill of skills) {
    if (!identifierPattern.test(skill)) {
      throw new Error(`${label} contains an invalid skill name`);
    }
  }
  if (skills.length === 0) {
    throw new Error(`${label} contains no direct SKILL.md-backed skills`);
  }
  return skills;
}

function projectedSkillSnapshot(skillsRoot, label) {
  const source = snapshotRegularTree(skillsRoot, treeLimits, {
    ignoredEntryNames: [".plugin-eval"],
  });
  const skills = directSkillNames(source, label);
  const selected = new Set(skills);
  const files = new Map();
  for (const [relative, contents] of source.files) {
    const normalized = slashPath(relative);
    const [direct] = normalized.split("/");
    if (!selected.has(direct)) continue;
    if (hasForbiddenSegment(normalized)) {
      throw new Error(`${label} contains forbidden runtime content`);
    }
    files.set(relative, contents);
  }
  return {
    digest: sha256TreeSnapshot(files),
    files,
    skills,
  };
}

function pluginProjection(plugin) {
  const sourceRoot = path.resolve(root, plugin.sourcePath);
  const expectedSourceRoot = path.join(root, "plugins", plugin.name);
  if (sourceRoot !== expectedSourceRoot) {
    throw new Error(`plugin source binding is invalid: ${plugin.name}`);
  }
  assertRegularDirectory(sourceRoot, `plugin source ${plugin.name}`);
  const manifestFile = path.join(sourceRoot, ".codex-plugin/plugin.json");
  const sourceManifest = readJsonRegular(
    manifestFile,
    `Codex plugin manifest ${plugin.name}`,
  ).value;
  if (
    sourceManifest?.name !== plugin.name ||
    sourceManifest?.version !== plugin.version
  ) {
    throw new Error(`Codex plugin manifest binding is invalid: ${plugin.name}`);
  }
  const skills = projectedSkillSnapshot(
    path.join(sourceRoot, "skills"),
    `plugin ${plugin.name}`,
  );
  return {
    ...plugin,
    manifest: Buffer.from(
      `${canonicalJson(sanitizedPluginManifest(plugin), 2)}\n`,
      "utf8",
    ),
    skills,
  };
}

function loadRuntimeProjections(contract) {
  const marketplace = readJsonRegular(
    marketplaceFile,
    "Codex marketplace manifest",
  ).value;
  const selectedByMode = new Map();
  const selectedPlugins = new Map();
  for (const condition of contract.conditions) {
    const selected = selectMarketplacePlugins(
      contract,
      marketplace,
      condition.id,
    );
    selectedByMode.set(condition.id, selected);
    for (const plugin of selected) selectedPlugins.set(plugin.name, plugin);
  }
  const projections = new Map(
    [...selectedPlugins.values()].map((plugin) => [
      plugin.name,
      pluginProjection(plugin),
    ]),
  );
  return { projections, selectedByMode };
}

function assertPinnedCodexBinary(executionSurface) {
  const binary = process.env.CODE_QUALITY_CODEX_REAL_BIN;
  if (
    typeof binary !== "string" ||
    !path.isAbsolute(binary) ||
    path.resolve(binary) !== binary
  ) {
    throw new Error("pinned Codex binary path must be absolute and canonical");
  }
  const stat = fs.lstatSync(binary, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    fs.realpathSync(binary) !== binary ||
    (stat.mode & 0o111) === 0 ||
    stat.size < 1 ||
    stat.size > 512 * 1024 * 1024
  ) {
    throw new Error("pinned Codex binary must be a canonical executable file");
  }
  const actual = sha256(fs.readFileSync(binary));
  if (actual !== executionSurface.codexBinarySha256) {
    throw new Error("Codex binary digest does not match the pinned digest");
  }
  return { binary, digest: actual };
}

function runPinnedCodex(binary, arguments_, scratch) {
  return spawnSync(binary, arguments_, {
    cwd: path.join(scratch, "work"),
    encoding: "utf8",
    env: {
      CODEX_HOME: path.join(scratch, "codex-home"),
      HOME: path.join(scratch, "home"),
      LANG: "C.UTF-8",
      LC_ALL: "C.UTF-8",
      NO_COLOR: "1",
      PATH: "/usr/bin:/bin",
      TERM: "dumb",
      TMPDIR: path.join(scratch, "tmp"),
    },
    killSignal: "SIGKILL",
    maxBuffer: 512 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 30_000,
  });
}

function materializeSystemSkills(stagingRoot, executionSurface) {
  const { binary, digest } = assertPinnedCodexBinary(executionSurface);
  const scratch = path.join(stagingRoot, ".codex-system-materialization");
  fs.mkdirSync(scratch, { mode: 0o700 });
  for (const directory of ["codex-home", "home", "tmp", "work"]) {
    fs.mkdirSync(path.join(scratch, directory), { mode: 0o700 });
  }

  const versionResult = runPinnedCodex(binary, ["--version"], scratch);
  if (
    versionResult.status !== 0 ||
    versionResult.stdout.trim() !== executionSurface.codexVersion
  ) {
    throw new Error("Codex binary version does not match the pinned version");
  }
  const materialization = runPinnedCodex(
    binary,
    [
      "debug",
      "prompt-input",
      "-c",
      "features.plugins=false",
      "-c",
      "features.goals=false",
      "-c",
      "features.shell_snapshot=false",
      "-c",
      'history.persistence="none"',
      "--",
      "materialize bundled skills",
    ],
    scratch,
  );
  if (materialization.status !== 0) {
    throw new Error(
      `Codex bundled-skill materialization failed: ${materialization.error?.message || materialization.stderr || materialization.stdout}`,
    );
  }
  if (sha256(fs.readFileSync(binary)) !== digest) {
    throw new Error(
      "Codex binary changed during bundled-skill materialization",
    );
  }

  const systemRoot = path.join(scratch, "codex-home/skills/.system");
  const snapshot = snapshotRegularTree(systemRoot, treeLimits);
  const marker = snapshot.files.get(systemMarkerName);
  if (!marker || !systemMarkerPattern.test(marker.toString("utf8"))) {
    throw new Error("Codex bundled-skill marker is invalid");
  }
  const skills = directSkillNames(snapshot, "Codex bundled skills");
  const allowed = new Set([systemMarkerName, ...skills]);
  for (const relative of snapshot.files.keys()) {
    const normalized = slashPath(relative);
    const [direct] = normalized.split("/");
    if (!allowed.has(direct) || hasForbiddenSegment(normalized)) {
      throw new Error("Codex bundled-skill projection contains extra content");
    }
  }
  for (const relative of snapshot.directories) {
    const normalized = slashPath(relative);
    if (normalized === "") continue;
    if (!allowed.has(normalized.split("/")[0])) {
      throw new Error("Codex bundled-skill projection contains extra content");
    }
  }
  fs.rmSync(scratch, { force: true, recursive: true });
  return { ...snapshot, skills };
}

function writePluginProjection(destination, projection) {
  fs.mkdirSync(path.join(destination, ".codex-plugin"), {
    mode: 0o700,
    recursive: true,
  });
  fs.writeFileSync(
    path.join(destination, ".codex-plugin/plugin.json"),
    projection.manifest,
    { mode: 0o600 },
  );
  writeTreeSnapshot(projection.skills, path.join(destination, "skills"));
}

function prepareHome({
  executionSurface,
  physicalCodexHome,
  plugins,
  projections,
  system,
}) {
  fs.mkdirSync(physicalCodexHome, { mode: 0o700 });
  fs.writeFileSync(
    path.join(physicalCodexHome, evalHomeMarkerName),
    evalHomeMarkerContents,
    { mode: 0o600 },
  );
  fs.writeFileSync(
    path.join(physicalCodexHome, executionSurfaceName),
    `${canonicalJson(executionSurface, 2)}\n`,
    { mode: 0o600 },
  );
  fs.writeFileSync(
    path.join(physicalCodexHome, "config.toml"),
    runtimeConfigForPlugins(plugins),
    { mode: 0o600 },
  );

  fs.mkdirSync(path.join(physicalCodexHome, "skills"), { mode: 0o700 });
  writeTreeSnapshot(system, path.join(physicalCodexHome, "skills/.system"));

  const marketplaceRoot = path.join(physicalCodexHome, "marketplace");
  fs.mkdirSync(path.join(marketplaceRoot, ".agents/plugins"), {
    mode: 0o700,
    recursive: true,
  });
  fs.writeFileSync(
    path.join(marketplaceRoot, ".agents/plugins/marketplace.json"),
    `${canonicalJson(sanitizedMarketplaceManifest(plugins), 2)}\n`,
    { mode: 0o600 },
  );

  for (const plugin of plugins) {
    const projection = projections.get(plugin.name);
    if (!projection || projection.version !== plugin.version) {
      throw new Error(
        `runtime plugin projection is unavailable: ${plugin.name}`,
      );
    }
    writePluginProjection(
      path.join(marketplaceRoot, "plugins", plugin.name),
      projection,
    );
    writePluginProjection(
      path.join(
        physicalCodexHome,
        "plugins/cache/ai-plugins",
        plugin.name,
        plugin.version,
      ),
      projection,
    );
  }
}

function chmodPrivateTree(entry) {
  walkTree(entry, (candidate, stat) => {
    if (stat.isDirectory()) fs.chmodSync(candidate, 0o700);
    else if (stat.isFile()) fs.chmodSync(candidate, 0o600);
  });
}

function buildRuntimeManifest(
  inputs,
  runtimeRoot,
  physicalRoot,
  { executionSurface, projections, selectedByMode, system },
) {
  const conditions = new Map(
    inputs.contract.conditions.map((condition) => [condition.id, condition]),
  );
  const matrixBindings = inputs.rows.map((row) => {
    if (
      typeof row.fixtureDigest !== "string" ||
      !sha256Pattern.test(row.fixtureDigest)
    ) {
      throw new Error("workspace fixtureDigest must be a SHA-256 digest");
    }
    return {
      baselineOid: row.baselineOid,
      caseId: row.caseId,
      fixtureDigest: row.fixtureDigest,
      mode: row.mode,
      sample: row.sample,
      workspace: row.workspace,
    };
  });
  const matrixHash = hashCanonical({
    contractSha256: inputs.contractSha256,
    rows: matrixBindings,
    schemaVersion: 1,
    workspaceManifestSha256: inputs.workspaceManifestSha256,
  });
  const runId = inputs.manifest.runId;

  const rows = inputs.rows.map((row) => {
    const condition = conditions.get(row.mode);
    if (!condition) throw new Error(`unknown runtime condition: ${row.mode}`);
    const relativeRow = path.join(row.caseId, `sample-${row.sample}`, row.mode);
    const physicalRow = path.join(physicalRoot, relativeRow);
    const physicalCodexHome = path.join(physicalRow, "codex-home");
    const physicalTmp = path.join(physicalRow, "tmp");
    fs.mkdirSync(physicalRow, { mode: 0o700, recursive: true });
    fs.mkdirSync(physicalTmp, { mode: 0o700 });
    const plugins = selectedByMode.get(condition.id);
    if (!plugins) {
      throw new Error(
        `runtime plugin selection is unavailable: ${condition.id}`,
      );
    }
    prepareHome({
      executionSurface,
      physicalCodexHome,
      plugins,
      projections,
      system,
    });
    const { availableSkills, compositionHash } = measureRuntimeEvidence({
      codexHome: physicalCodexHome,
      mode: row.mode,
    });
    const inputHash = inputHashFor(row, inputs.contract.id);

    return {
      availableSkills,
      baselineOid: row.baselineOid,
      caseId: row.caseId,
      codexHome: path.join(runtimeRoot, relativeRow, "codex-home"),
      codexTmp: path.join(runtimeRoot, relativeRow, "tmp"),
      compositionHash,
      contractSha256: inputs.contractSha256,
      fixtureDigest: row.fixtureDigest,
      inputHash,
      matrixHash,
      mode: row.mode,
      runId,
      sample: row.sample,
      workspace: row.workspace,
      workspaceManifestSha256: inputs.workspaceManifestSha256,
    };
  });

  return {
    benchmarkId: inputs.contract.id,
    contractSha256: inputs.contractSha256,
    matrixHash,
    rows,
    runId,
    runtimeRoot,
    schemaVersion: 1,
    workspaceManifest: inputs.manifestPath,
    workspaceManifestSha256: inputs.workspaceManifestSha256,
  };
}

let stagingRoot;
try {
  const arguments_ = parseArguments(process.argv.slice(2));
  const inputs = loadValidatedInputs(arguments_.workspaceManifest);
  assertRequestedRuntimePathHasNoSymlinks(arguments_.runtimeRoot);
  const runtimeRoot = canonicalProspectivePath(arguments_.runtimeRoot);
  assertRuntimePathIsIsolated(runtimeRoot, inputs);
  const existingRootWasEmpty = assertRuntimeRootIsUnused(runtimeRoot);

  const parent = path.dirname(runtimeRoot);
  stagingRoot = fs.mkdtempSync(
    path.join(parent, `.${path.basename(runtimeRoot)}.staging-`),
  );
  fs.chmodSync(stagingRoot, 0o700);
  assertRuntimePathIsIsolated(stagingRoot, inputs);
  fs.writeFileSync(
    path.join(stagingRoot, runtimeMarkerName),
    runtimeMarkerContents,
    { mode: 0o600 },
  );

  const executionSurface = executionSurfaceFromEnvironment();
  const projections = loadRuntimeProjections(inputs.contract);
  const system = materializeSystemSkills(stagingRoot, executionSurface);

  const runtimeManifest = buildRuntimeManifest(
    inputs,
    runtimeRoot,
    stagingRoot,
    {
      executionSurface,
      projections: projections.projections,
      selectedByMode: projections.selectedByMode,
      system,
    },
  );
  const serialized = `${canonicalJson(runtimeManifest, 2)}\n`;
  fs.writeFileSync(path.join(stagingRoot, "manifest.json"), serialized, {
    mode: 0o600,
  });
  assertTreeHasNoSymlinksOrCredentials(stagingRoot);
  chmodPrivateTree(stagingRoot);
  if (existingRootWasEmpty) {
    const existingStat = fs.lstatSync(runtimeRoot, { throwIfNoEntry: false });
    if (
      !existingStat ||
      !existingStat.isDirectory() ||
      existingStat.isSymbolicLink() ||
      fs.readdirSync(runtimeRoot).length !== 0
    ) {
      throw new Error(
        `runtime root changed during preparation: ${runtimeRoot}`,
      );
    }
    fs.rmdirSync(runtimeRoot);
  } else if (fs.lstatSync(runtimeRoot, { throwIfNoEntry: false })) {
    throw new Error(`runtime root appeared during preparation: ${runtimeRoot}`);
  }
  fs.renameSync(stagingRoot, runtimeRoot);
  stagingRoot = undefined;
  process.stdout.write(serialized);
} catch (error) {
  if (stagingRoot !== undefined) {
    fs.rmSync(stagingRoot, { force: true, recursive: true });
  }
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
}
