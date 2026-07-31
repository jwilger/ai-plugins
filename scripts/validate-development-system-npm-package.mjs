#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const relativePackage = "plugins/development-system";
const packageRoot = path.join(root, relativePackage);
const manifest = JSON.parse(
  fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"),
);
const inventory = JSON.parse(
  fs.readFileSync(path.join(root, ".agents/plugins/pi-support.json"), "utf8"),
).packages.find((entry) => entry.name === "development-system");
if (!inventory) throw new Error("development_system.npm_inventory_missing");

const packed = spawnSync(
  "npm",
  ["pack", `./${relativePackage}`, "--dry-run", "--json"],
  { cwd: root, encoding: "utf8", maxBuffer: 10 * 1024 * 1024 },
);
if (packed.status !== 0)
  throw new Error(`development_system.npm_pack_failed ${packed.stderr}`);
const reports = JSON.parse(packed.stdout);
if (!Array.isArray(reports) || reports.length !== 1)
  throw new Error("development_system.npm_pack_report_invalid");
const report = reports[0];
const files = new Map(report.files.map((entry) => [entry.path, entry]));

const required = new Set([
  "package.json",
  "README.md",
  inventory.extension.replace(/^\.\//, ""),
  ...inventory.skills.map((skill) => `skills/${skill}/SKILL.md`),
  ...inventory.componentEntrypoints.map((entry) => entry.replace(/^\.\//, "")),
]);
for (const formula of fs
  .readdirSync(path.join(packageRoot, "formulas"))
  .filter((name) => name.endsWith(".formula.toml")))
  required.add(`formulas/${formula}`);
for (const toolFile of [
  "bin/bd",
  "bin/dolt",
  "bin/install-development-tool.mjs",
  "bin/tool-releases.json",
  "bin/migrate-tiber-to-beads.mjs",
])
  required.add(toolFile);
for (const component of ["development-discipline"]) {
  const release = JSON.parse(
    fs.readFileSync(
      path.join(packageRoot, `components/${component}/release-binaries.json`),
      "utf8",
    ),
  );
  for (const binary of release.binaries)
    required.add(`components/${component}/${binary.path}`);
}
for (const requiredFile of required) {
  if (!files.has(requiredFile))
    throw new Error(
      `development_system.npm_required_file_missing path=${requiredFile}`,
    );
}

for (const [file, metadata] of files) {
  if (
    file.includes("/rust/") ||
    file.includes("agent-sources/") ||
    /(^|\/)\.env(?:\.|$)/.test(file) ||
    /(^|\/)(?:auth|credentials)\.json$/.test(file)
  )
    throw new Error(`development_system.npm_forbidden_file path=${file}`);
  if (
    ((file.startsWith("bin/") && !file.endsWith(".json")) ||
      file.includes("/dist/")) &&
    (metadata.mode & 0o111) === 0
  )
    throw new Error(
      `development_system.npm_executable_mode_missing path=${file}`,
    );
}

if (manifest.name !== "@jwilger/development-system-pi")
  throw new Error("development_system.npm_package_name_invalid");
if (manifest.private !== true)
  throw new Error("development_system.npm_package_must_be_private");
if (Object.hasOwn(manifest, "publishConfig"))
  throw new Error("development_system.npm_publish_config_forbidden");
if (report.name !== manifest.name || report.version !== manifest.version)
  throw new Error("development_system.npm_pack_identity_mismatch");
if (
  !Number.isFinite(report.unpackedSize) ||
  report.unpackedSize > 100 * 1024 * 1024
)
  throw new Error(
    `development_system.npm_package_size_exceeded bytes=${report.unpackedSize}`,
  );

process.stdout.write(
  `${JSON.stringify({ ok: true, name: report.name, version: report.version, files: report.entryCount, packedBytes: report.size, unpackedBytes: report.unpackedSize })}\n`,
);
