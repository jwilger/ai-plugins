#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const markerName = ".ai-plugins-gpt56-workspace";
const markerContents = "ai-plugins GPT-5.6 benchmark workspace\n";

function parseArgs(argv) {
  if (argv.length === 0 || !argv[0].trim()) {
    throw new Error(
      "Usage: node scripts/evals/prepare-gpt56-workspace.mjs <workspace> [--forbid-overlap <path>]...",
    );
  }

  const workspace = path.resolve(argv.shift());
  const protectedPaths = [];
  let checkOnly = false;
  while (argv.length > 0) {
    const option = argv.shift();
    if (option === "--check") {
      checkOnly = true;
      continue;
    }
    const protectedPath = argv.shift();
    if (
      option !== "--forbid-overlap" ||
      typeof protectedPath !== "string" ||
      protectedPath.trim() === ""
    ) {
      throw new Error(
        "Usage: node scripts/evals/prepare-gpt56-workspace.mjs <workspace> [--forbid-overlap <path>]...",
      );
    }
    protectedPaths.push(path.resolve(protectedPath));
  }

  return { checkOnly, protectedPaths, workspace };
}

function realPathIfExists(entry) {
  try {
    return fs.realpathSync(entry);
  } catch {
    return path.resolve(entry);
  }
}

function isSameOrAncestor(ancestor, descendant) {
  const relative = path.relative(ancestor, descendant);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

function pathsOverlap(first, second) {
  return isSameOrAncestor(first, second) || isSameOrAncestor(second, first);
}

function isEmptyOrOwned(workspace) {
  if (!fs.existsSync(workspace)) return true;
  if (!fs.statSync(workspace).isDirectory()) return false;

  const entries = fs.readdirSync(workspace);
  if (entries.length === 0) return true;

  const marker = path.join(workspace, markerName);
  if (!fs.existsSync(marker)) return false;

  return fs.readFileSync(marker, "utf8") === markerContents;
}

function isEmptyDirectory(entry) {
  return (
    fs.existsSync(entry) &&
    fs.statSync(entry).isDirectory() &&
    fs.readdirSync(entry).length === 0
  );
}

try {
  const { checkOnly, protectedPaths, workspace } = parseArgs(
    process.argv.slice(2),
  );
  const initializeInPlace = isEmptyDirectory(workspace);
  const realWorkspace = realPathIfExists(workspace);
  for (const protectedPath of protectedPaths) {
    if (pathsOverlap(realWorkspace, realPathIfExists(protectedPath))) {
      throw new Error(
        `GPT-5.6 benchmark workspace overlaps protected path: ${protectedPath}`,
      );
    }
  }

  if (!isEmptyOrOwned(workspace)) {
    throw new Error(
      `refusing to replace unowned GPT-5.6 benchmark workspace: ${workspace}`,
    );
  }

  if (checkOnly) process.exit(0);

  if (!initializeInPlace) {
    fs.rmSync(workspace, { recursive: true, force: true });
  }
  fs.mkdirSync(workspace, { recursive: true });
  fs.writeFileSync(path.join(workspace, markerName), markerContents, {
    mode: 0o600,
  });
  console.log(`prepared GPT-5.6 benchmark workspace: ${workspace}`);
} catch (error) {
  console.error(error.message);
  process.exit(2);
}
