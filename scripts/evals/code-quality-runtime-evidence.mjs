#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import {
  canonicalJson,
  credentialNames,
  evalHomeMarkerContents,
  evalHomeMarkerName,
  executionSurfaceName,
  hashCanonical,
  identifierPattern,
  parseExecutionSurface,
  runtimeConfigForPlugins,
  sanitizedMarketplaceManifest,
  sanitizedPluginManifest,
  selectMarketplacePlugins,
} from "./code-quality-runtime-contract.mjs";
import {
  sha256TreeSnapshot,
  snapshotRegularTree,
} from "./code-quality-tree-hash.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const benchmarkDirectory = path.join(
  root,
  "evals/benchmarks/downstream-code-quality",
);
const benchmarkConfig = path.join(benchmarkDirectory, "promptfooconfig.yaml");
const contractFile = path.join(benchmarkDirectory, "benchmark.json");
const marketplaceFile = path.join(root, ".agents/plugins/marketplace.json");
const require = createRequire(import.meta.url);
const { loadPromptfooSurface } = require(
  path.join(benchmarkDirectory, "benchmark-inputs.cjs"),
);

const systemRoot = "skills/.system";
const systemMarkerName = ".codex-system-skills.marker";
const systemMarkerPattern = /^[0-9a-f]{16}\n$/;
const limits = Object.freeze({
  directories: 4096,
  fileBytes: 1024 * 1024,
  files: 4096,
  pathBytes: 1024,
  totalBytes: 32 * 1024 * 1024,
});

export class RuntimeEvidenceError extends Error {
  constructor(category, code) {
    super(code.replaceAll("-", " "));
    this.name = "RuntimeEvidenceError";
    this.category = category;
    this.code = code;
  }
}

function fail(category, code) {
  throw new RuntimeEvidenceError(category, code);
}

function readJson(file, unavailableCode) {
  let bytes;
  try {
    bytes = fs.readFileSync(file, "utf8");
  } catch {
    fail("operational", unavailableCode);
  }
  try {
    return JSON.parse(bytes);
  } catch {
    fail("provenance", `${unavailableCode}-invalid`);
  }
}

function expectedPlugins(mode) {
  const contract = readJson(contractFile, "benchmark-contract-unavailable");
  const marketplace = readJson(
    marketplaceFile,
    "codex-marketplace-unavailable",
  );
  try {
    return selectMarketplacePlugins(contract, marketplace, mode);
  } catch (error) {
    const code = String(error?.message || "runtime-condition-invalid");
    if (/^[a-z0-9-]+$/.test(code)) fail("provenance", code);
    fail("provenance", "runtime-condition-invalid");
  }
}

function slashPath(value) {
  return value.split(path.sep).join("/");
}

function ancestorsOf(files) {
  const result = new Set([""]);
  for (const file of files) {
    let current = path.posix.dirname(slashPath(file));
    while (current !== ".") {
      result.add(current);
      current = path.posix.dirname(current);
    }
  }
  return [...result].sort();
}

function exactStrings(left, right) {
  return JSON.stringify([...left].sort()) === JSON.stringify([...right].sort());
}

function assertNoCredentials(snapshot) {
  for (const relative of [...snapshot.directories, ...snapshot.files.keys()]) {
    if (
      slashPath(relative)
        .split("/")
        .some(
          (segment) =>
            credentialNames.has(segment) || segment === ".plugin-eval",
        )
    ) {
      fail(
        "provenance",
        slashPath(relative).split("/").includes(".plugin-eval")
          ? "forbidden-runtime-eval-artifact"
          : "forbidden-runtime-credential",
      );
    }
  }
}

function exactCanonicalJson(bytes, expected, invalidCode) {
  const expectedBytes = `${canonicalJson(expected, 2)}\n`;
  if (bytes.toString("utf8") !== expectedBytes) {
    fail("provenance", invalidCode);
  }
}

function parseExecutionSurfaceFile(bytes) {
  let parsed;
  try {
    parsed = JSON.parse(bytes.toString("utf8"));
  } catch {
    fail("provenance", "runtime-execution-surface-invalid");
  }
  let surface;
  try {
    surface = parseExecutionSurface(parsed);
  } catch {
    fail("provenance", "runtime-execution-surface-invalid");
  }
  exactCanonicalJson(bytes, surface, "runtime-execution-surface-invalid");
  return surface;
}

function validateDisposableAuth(codexHome) {
  const authPath = path.join(codexHome, "auth.json");
  const stat = fs.lstatSync(authPath, { throwIfNoEntry: false });
  if (!stat) return;
  try {
    if (
      !stat.isFile() ||
      stat.isSymbolicLink() ||
      stat.uid !== process.getuid() ||
      (stat.mode & 0o077) !== 0 ||
      fs.realpathSync(authPath) !== authPath ||
      stat.size < 2 ||
      stat.size > 64 * 1024
    ) {
      fail("provenance", "runtime-disposable-auth-invalid");
    }
    const auth = JSON.parse(fs.readFileSync(authPath, "utf8"));
    if (
      auth?.auth_mode !== "chatgpt" ||
      !auth.tokens ||
      typeof auth.tokens !== "object" ||
      Array.isArray(auth.tokens)
    ) {
      fail("provenance", "runtime-disposable-auth-invalid");
    }
  } catch (error) {
    if (error instanceof RuntimeEvidenceError) throw error;
    fail("provenance", "runtime-disposable-auth-invalid");
  }
}

function validateSkillTree(snapshot, skillsRoot, namespace, allowedFiles) {
  const normalizedRoot = slashPath(skillsRoot);
  const skillDirectories = snapshot.directories
    .map(slashPath)
    .filter((candidate) => {
      if (!candidate.startsWith(`${normalizedRoot}/`)) return false;
      return candidate.slice(normalizedRoot.length + 1).split("/").length === 1;
    })
    .sort();
  if (skillDirectories.length === 0) {
    fail("provenance", "projected-plugin-has-no-skills");
  }
  const availableSkills = [];
  for (const skillDirectory of skillDirectories) {
    const skill = skillDirectory.slice(normalizedRoot.length + 1);
    if (!identifierPattern.test(skill)) {
      fail("provenance", "projected-skill-name-invalid");
    }
    const skillFile = `${skillDirectory}/SKILL.md`;
    if (!snapshot.files.has(skillFile)) {
      fail("provenance", "projected-skill-is-missing-skill-md");
    }
    if (namespace) availableSkills.push(`${namespace}:${skill}`);
    for (const file of snapshot.files.keys()) {
      const normalized = slashPath(file);
      if (normalized.startsWith(`${skillDirectory}/`)) {
        allowedFiles.add(normalized);
      }
    }
  }
  return {
    availableSkills,
    skillNames: skillDirectories.map((directory) =>
      directory.slice(normalizedRoot.length + 1),
    ),
  };
}

function relativeTreeFiles(snapshot, treeRoot) {
  const prefix = `${slashPath(treeRoot)}/`;
  return new Map(
    [...snapshot.files]
      .map(([relative, contents]) => [slashPath(relative), contents])
      .filter(([relative]) => relative.startsWith(prefix))
      .map(([relative, contents]) => [relative.slice(prefix.length), contents]),
  );
}

function exactTreeCopies(left, right) {
  if (!exactStrings(left.keys(), right.keys())) return false;
  for (const [relative, contents] of left) {
    if (!contents.equals(right.get(relative))) return false;
  }
  return true;
}

function selectedSkillFiles(snapshot, treeRoot, skillNames) {
  const selected = new Set(skillNames);
  return new Map(
    [...relativeTreeFiles(snapshot, treeRoot)].filter(([relative]) =>
      selected.has(relative.split("/")[0]),
    ),
  );
}

function validateSystemSkills(snapshot, allowedFiles) {
  const markerPath = `${systemRoot}/${systemMarkerName}`;
  const marker = snapshot.files.get(markerPath);
  if (!marker || !systemMarkerPattern.test(marker.toString("utf8"))) {
    fail("provenance", "runtime-system-skill-marker-invalid");
  }
  allowedFiles.add(markerPath);
  const result = validateSkillTree(
    snapshot,
    systemRoot,
    "codex-system",
    allowedFiles,
  );
  return {
    availableSkills: result.availableSkills,
    treeSha256: sha256TreeSnapshot(relativeTreeFiles(snapshot, systemRoot)),
  };
}

function validatePluginProjection(snapshot, plugin, allowedFiles) {
  const cacheRoot = `plugins/cache/ai-plugins/${plugin.name}/${plugin.version}`;
  const sourceRoot = `marketplace/plugins/${plugin.name}`;
  const expectedManifest = sanitizedPluginManifest(plugin);
  for (const projectionRoot of [cacheRoot, sourceRoot]) {
    const manifestPath = `${projectionRoot}/.codex-plugin/plugin.json`;
    const manifestBytes = snapshot.files.get(manifestPath);
    if (!manifestBytes) {
      fail("operational", "projected-plugin-manifest-unavailable");
    }
    exactCanonicalJson(
      manifestBytes,
      expectedManifest,
      "projected-plugin-manifest-invalid",
    );
    allowedFiles.add(manifestPath);
  }
  const cacheSkillsRoot = `${cacheRoot}/skills`;
  const sourceSkillsRoot = `${sourceRoot}/skills`;
  const cacheSkills = validateSkillTree(
    snapshot,
    cacheSkillsRoot,
    plugin.name,
    allowedFiles,
  );
  const sourceSkills = validateSkillTree(
    snapshot,
    sourceSkillsRoot,
    null,
    allowedFiles,
  );
  if (
    !exactStrings(cacheSkills.skillNames, sourceSkills.skillNames) ||
    !exactTreeCopies(
      selectedSkillFiles(snapshot, cacheSkillsRoot, cacheSkills.skillNames),
      selectedSkillFiles(snapshot, sourceSkillsRoot, sourceSkills.skillNames),
    )
  ) {
    fail("provenance", "runtime-plugin-projections-differ");
  }
  return cacheSkills.availableSkills;
}

function validateProjection(snapshot, plugins) {
  assertNoCredentials(snapshot);
  const files = [...snapshot.files.keys()].map(slashPath).sort();
  const directories = snapshot.directories.map(slashPath).sort();
  const marker = snapshot.files.get(evalHomeMarkerName);
  if (!marker || marker.toString("utf8") !== evalHomeMarkerContents) {
    fail("operational", "runtime-home-marker-unavailable");
  }
  const config = snapshot.files.get("config.toml");
  if (!config) fail("operational", "runtime-config-unavailable");
  if (config.toString("utf8") !== runtimeConfigForPlugins(plugins)) {
    fail("provenance", "runtime-config-invalid");
  }
  const executionSurfaceBytes = snapshot.files.get(executionSurfaceName);
  if (!executionSurfaceBytes) {
    fail("operational", "runtime-execution-surface-unavailable");
  }
  const executionSurface = parseExecutionSurfaceFile(executionSurfaceBytes);
  const marketplaceManifestPath =
    "marketplace/.agents/plugins/marketplace.json";
  const marketplaceManifest = snapshot.files.get(marketplaceManifestPath);
  if (!marketplaceManifest) {
    fail("operational", "runtime-marketplace-manifest-unavailable");
  }
  exactCanonicalJson(
    marketplaceManifest,
    sanitizedMarketplaceManifest(plugins),
    "runtime-marketplace-manifest-invalid",
  );

  const allowedFiles = new Set([
    evalHomeMarkerName,
    executionSurfaceName,
    "config.toml",
    marketplaceManifestPath,
  ]);
  const system = validateSystemSkills(snapshot, allowedFiles);
  const availableSkills = [...system.availableSkills];
  for (const plugin of plugins) {
    availableSkills.push(
      ...validatePluginProjection(snapshot, plugin, allowedFiles),
    );
  }

  if (!exactStrings(files, allowedFiles)) {
    fail("provenance", "runtime-projection-file-set-is-not-exact");
  }
  if (!exactStrings(directories, ancestorsOf(files))) {
    fail("provenance", "runtime-projection-directory-set-is-not-exact");
  }
  availableSkills.sort();
  if (new Set(availableSkills).size !== availableSkills.length) {
    fail("provenance", "runtime-projection-skills-are-not-unique");
  }
  return {
    availableSkills,
    config,
    executionSurface,
    systemSkillsTreeSha256: system.treeSha256,
  };
}

function promptfooSurface(mode) {
  let surface;
  try {
    surface = loadPromptfooSurface(benchmarkConfig);
  } catch (error) {
    if (error?.code === "ENOENT" || error?.code === "EACCES") {
      fail("operational", "promptfoo-config-unavailable");
    }
    fail("provenance", "promptfoo-config-surface-invalid");
  }
  const provider = surface.providers[mode];
  if (!provider) fail("provenance", "promptfoo-provider-binding-missing");
  return { provider, ...surface };
}

export function measureRuntimeEvidence({
  codexHome,
  mode,
  phase = "pre-turn",
}) {
  if (
    typeof codexHome !== "string" ||
    !path.isAbsolute(codexHome) ||
    path.resolve(codexHome) !== codexHome ||
    typeof mode !== "string" ||
    !identifierPattern.test(mode) ||
    !["pre-turn", "post-turn"].includes(phase)
  ) {
    fail("provenance", "runtime-evidence-arguments-invalid");
  }
  let snapshot;
  try {
    validateDisposableAuth(codexHome);
    snapshot = snapshotRegularTree(codexHome, limits, {
      ignoredRootEntries: ["auth.json"],
    });
  } catch (error) {
    if (error instanceof RuntimeEvidenceError) throw error;
    if (!fs.lstatSync(codexHome, { throwIfNoEntry: false })) {
      fail("operational", "runtime-codex-home-unavailable");
    }
    if (error?.message === "tree-directory-invalid") {
      fail("operational", "runtime-codex-home-unreadable");
    }
    fail("provenance", "runtime-projection-tree-invalid");
  }
  const plugins = expectedPlugins(mode);
  const { availableSkills, config, executionSurface, systemSkillsTreeSha256 } =
    validateProjection(snapshot, plugins);
  const surface = promptfooSurface(mode);
  const compositionHash = hashCanonical({
    availableSkills,
    codexConfig: config.toString("utf8"),
    codexHomeDirectories: snapshot.directories.map(slashPath).sort(),
    codexHomeTreeSha256: snapshot.digest,
    executionSurface,
    promptfooConfigSha256: surface.configSha256,
    promptfooProvider: surface.provider,
    schemaVersion: 3,
    systemSkillsTreeSha256,
  });
  return { availableSkills, compositionHash };
}

function parseArguments(argv) {
  if (
    argv.length !== 6 ||
    argv[0] !== "--codex-home" ||
    argv[2] !== "--mode" ||
    argv[4] !== "--phase"
  ) {
    fail("operational", "runtime-evidence-cli-usage-invalid");
  }
  return { codexHome: argv[1], mode: argv[3], phase: argv[5] };
}

if (process.argv[1] && path.resolve(process.argv[1]) === import.meta.filename) {
  try {
    process.stdout.write(
      `${JSON.stringify(measureRuntimeEvidence(parseArguments(process.argv.slice(2))))}\n`,
    );
  } catch (error) {
    const classified =
      error instanceof RuntimeEvidenceError
        ? error
        : new RuntimeEvidenceError("operational", "runtime-evidence-failed");
    process.stderr.write(
      `code-quality-runtime-evidence:${classified.category}:${classified.code}\n`,
    );
    process.exitCode = 2;
  }
}
