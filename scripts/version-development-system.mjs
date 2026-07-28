#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const root = path.resolve(import.meta.dirname, "..");
const packageFile = path.join(root, "plugins/development-system/package.json");

export function nextVersion(current, bump) {
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(current);
  if (!match)
    throw new Error(
      `development_system.release_version_invalid value=${current}`,
    );
  const [, majorText, minorText, patchText] = match;
  const major = Number(majorText);
  const minor = Number(minorText);
  const patch = Number(patchText);
  if (bump === "major") return `${major + 1}.0.0`;
  if (bump === "minor") return `${major}.${minor + 1}.0`;
  if (bump === "patch") return `${major}.${minor}.${patch + 1}`;
  if (bump === "current") return current;
  throw new Error(`development_system.release_bump_invalid value=${bump}`);
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8" });
  if (result.status !== 0)
    throw new Error(
      `development_system.release_command_failed command=${command} status=${result.status}\n${result.stderr}`,
    );
  return result.stdout;
}

export function applyVersion(bump) {
  const manifest = JSON.parse(fs.readFileSync(packageFile, "utf8"));
  const previous = manifest.version;
  const version = nextVersion(previous, bump);
  if (bump !== "current") {
    manifest.version = version;
    fs.writeFileSync(packageFile, `${JSON.stringify(manifest, null, 2)}\n`);
    run(process.execPath, [
      "scripts/sync-development-system-metadata.mjs",
      "--write",
    ]);
    run(process.execPath, ["scripts/generate-pi-support-docs.mjs", "--write"]);
  }
  return { previous, version, changed: previous !== version };
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  const bumpIndex = process.argv.indexOf("--bump");
  const bump = bumpIndex >= 0 ? process.argv[bumpIndex + 1] : undefined;
  if (!bump) {
    console.error(
      "usage: version-development-system.mjs --bump patch|minor|major|current",
    );
    process.exit(2);
  }
  const result = applyVersion(bump);
  process.stdout.write(`${JSON.stringify(result)}\n`);
}
