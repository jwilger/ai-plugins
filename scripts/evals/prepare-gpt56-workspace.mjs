#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  assertSafeGpt56WorkspaceLocation,
  gpt56WorkspaceMarkerContents,
  gpt56WorkspaceMarkerName,
  inspectGpt56Workspace,
} from "./gpt56-workspace-policy.mjs";

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

try {
  const { checkOnly, protectedPaths, workspace } = parseArgs(
    process.argv.slice(2),
  );
  assertSafeGpt56WorkspaceLocation(workspace);
  const workspaceState = inspectGpt56Workspace(workspace);
  const initializeInPlace = workspaceState.kind === "empty";
  const realWorkspace = realPathIfExists(workspace);
  for (const protectedPath of protectedPaths) {
    if (pathsOverlap(realWorkspace, realPathIfExists(protectedPath))) {
      throw new Error(
        `GPT-5.6 benchmark workspace overlaps protected path: ${protectedPath}`,
      );
    }
  }

  if (workspaceState.kind === "unowned") {
    throw new Error(
      `refusing to replace unowned GPT-5.6 benchmark workspace: ${workspace}`,
    );
  }

  if (checkOnly) process.exit(0);

  if (!initializeInPlace) {
    fs.rmSync(workspace, { recursive: true, force: true });
  }
  fs.mkdirSync(workspace, { recursive: true });
  fs.writeFileSync(
    path.join(workspace, gpt56WorkspaceMarkerName),
    gpt56WorkspaceMarkerContents,
    { mode: 0o600 },
  );
  console.log(`prepared GPT-5.6 benchmark workspace: ${workspace}`);
} catch (error) {
  console.error(error.message);
  process.exit(2);
}
