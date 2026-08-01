#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const canonicalManifestFile = path.join(
  root,
  "plugins/development-system/.codex-plugin/plugin.json",
);
const canonicalManifest = JSON.parse(fs.readFileSync(canonicalManifestFile, "utf8"));
const expectedVersion = canonicalManifest.version;
const targets = [
  ["plugins/development-system/.claude-plugin/plugin.json", (value) => value],
  [
    ".claude-plugin/marketplace.json",
    (value) =>
      value.plugins.find((plugin) => plugin.name === "development-system"),
  ],
  [
    ".agents/plugins/marketplace.json",
    (value) =>
      value.plugins.find((plugin) => plugin.name === "development-system"),
  ],
];

const cacheLauncherTargets = [
  "plugins/development-system/.mcp.json",
  "plugins/development-system/components/agentic-systems-engineering/.mcp.json",
  "plugins/development-system/components/development-discipline/.mcp.json",
];

const write = process.argv.includes("--write");
if (!write && !process.argv.includes("--check")) {
  console.error("usage: sync-development-system-metadata.mjs --check|--write");
  process.exit(2);
}

let changed = false;
for (const [relativeFile, select] of targets) {
  const file = path.join(root, relativeFile);
  const document = JSON.parse(fs.readFileSync(file, "utf8"));
  const record = select(document);
  if (!record)
    throw new Error(`development-system metadata missing from ${relativeFile}`);
  if (record.version === expectedVersion) continue;
  changed = true;
  if (!write) {
    console.error(
      `${relativeFile}: expected version ${expectedVersion}, found ${record.version}`,
    );
    continue;
  }
  record.version = expectedVersion;
  fs.writeFileSync(file, `${JSON.stringify(document, null, 2)}\n`);
}

const catalogFile = path.join(root, "README.md");
const catalogSource = fs.readFileSync(catalogFile, "utf8");
const catalogPattern =
  /(\| \[development-system\]\(plugins\/development-system\/README\.md\) \|[^\n]*\| )\d+\.\d+\.\d+(\s+\|)/;
const catalogMatch = catalogSource.match(catalogPattern);
if (!catalogMatch)
  throw new Error("development-system catalog row missing from README.md");
const catalogVersion = catalogSource.match(
  /\| \[development-system\]\(plugins\/development-system\/README\.md\) \|[^\n]*\| (\d+\.\d+\.\d+)\s+\|/,
)?.[1];
if (catalogVersion !== expectedVersion) {
  changed = true;
  if (!write) {
    console.error(
      `README.md: expected catalog version ${expectedVersion}, found ${catalogVersion}`,
    );
  } else {
    fs.writeFileSync(
      catalogFile,
      catalogSource.replace(catalogPattern, `$1${expectedVersion}$2`),
    );
  }
}

for (const relativeFile of cacheLauncherTargets) {
  const file = path.join(root, relativeFile);
  const source = fs.readFileSync(file, "utf8");
  const versions = [
    ...source.matchAll(/development-system\/([^/]+)\/bin\//g),
  ].map((match) => match[1]);
  if (versions.length === 0) {
    throw new Error(
      `development-system cache launcher missing from ${relativeFile}`,
    );
  }
  if (versions.every((version) => version === expectedVersion)) continue;
  changed = true;
  if (!write) {
    console.error(
      `${relativeFile}: expected cache version ${expectedVersion}, found ${[...new Set(versions)].join(", ")}`,
    );
    continue;
  }
  const updated = source.replace(
    /development-system\/[^/]+\/bin\//g,
    `development-system/${expectedVersion}/bin/`,
  );
  fs.writeFileSync(file, updated);
}

if (changed && !write) process.exit(1);
console.log(
  `development_system.metadata version=${expectedVersion} synchronized=${!changed || write}`,
);
