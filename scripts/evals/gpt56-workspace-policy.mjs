import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

export const gpt56WorkspaceMarkerName = ".ai-plugins-gpt56-workspace";
export const gpt56WorkspaceMarkerContents =
  "ai-plugins GPT-5.6 benchmark workspace\n";

function workspaceError(label, detail) {
  return new Error(`${label} ${detail}`);
}

function resolvedCandidate(workspace, label) {
  if (
    typeof workspace !== "string" ||
    workspace.trim() === "" ||
    workspace.includes("{{") ||
    workspace.includes("}}") ||
    !path.isAbsolute(workspace) ||
    path.resolve(workspace) !== workspace
  ) {
    throw workspaceError(label, "must be a resolved absolute path");
  }

  const missingSegments = [];
  let existingEntry = workspace;
  while (!fs.existsSync(existingEntry)) {
    const parent = path.dirname(existingEntry);
    if (parent === existingEntry) break;
    missingSegments.unshift(path.basename(existingEntry));
    existingEntry = parent;
  }

  const canonicalExistingEntry = fs.realpathSync(existingEntry);
  const canonicalWorkspace = path.join(
    canonicalExistingEntry,
    ...missingSegments,
  );
  if (canonicalWorkspace !== workspace) {
    throw workspaceError(label, "must not traverse symbolic-link aliases");
  }

  return workspace;
}

function nearestExistingDirectory(workspace) {
  let candidate = workspace;
  while (true) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isDirectory()) {
      return candidate;
    }
    const parent = path.dirname(candidate);
    if (parent === candidate) return parent;
    candidate = parent;
  }
}

function assertNoAncestorInstructions(workspace, label) {
  let ancestor = path.dirname(workspace);
  while (true) {
    const instructions = path.join(ancestor, "AGENTS.md");
    if (fs.existsSync(instructions)) {
      throw workspaceError(
        label,
        `is beneath an ancestor AGENTS.md: ${instructions}`,
      );
    }

    const parent = path.dirname(ancestor);
    if (parent === ancestor) return;
    ancestor = parent;
  }
}

function sanitizedGitEnvironment() {
  return {
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_OPTIONAL_LOCKS: "0",
    GIT_TERMINAL_PROMPT: "0",
    LC_ALL: "C",
    PATH: process.env.PATH ?? "/usr/bin:/bin",
  };
}

function assertOutsideGitCheckout(workspace, label) {
  let probeDirectory = nearestExistingDirectory(workspace);
  while (true) {
    const probe = spawnSync(
      "git",
      ["-C", probeDirectory, "rev-parse", "--show-toplevel"],
      {
        encoding: "utf8",
        env: sanitizedGitEnvironment(),
        stdio: ["ignore", "pipe", "pipe"],
      },
    );

    if (probe.error) {
      throw workspaceError(
        label,
        `could not verify Git checkout isolation: ${probe.error.message}`,
      );
    }
    if (probe.signal) {
      throw workspaceError(
        label,
        `could not verify Git checkout isolation: git exited on ${probe.signal}`,
      );
    }

    // A malformed .git entry can make Git exit nonzero even though there is no
    // checkout. Probe higher ancestors so it cannot hide a real outer checkout,
    // and reject only when rev-parse positively identifies one.
    if (probe.status === 0 && probe.stdout.trim() !== "") {
      throw workspaceError(
        label,
        `must be outside a Git checkout: ${probe.stdout.trim()}`,
      );
    }

    const parent = path.dirname(probeDirectory);
    if (parent === probeDirectory) return;
    probeDirectory = parent;
  }
}

export function assertSafeGpt56WorkspaceLocation(
  workspace,
  { label = "GPT-5.6 benchmark workspace" } = {},
) {
  const resolvedWorkspace = resolvedCandidate(workspace, label);
  assertNoAncestorInstructions(resolvedWorkspace, label);
  assertOutsideGitCheckout(resolvedWorkspace, label);
  return resolvedWorkspace;
}

export function inspectGpt56Workspace(workspace) {
  if (!fs.existsSync(workspace)) return { kind: "missing", entries: [] };
  if (!fs.statSync(workspace).isDirectory()) {
    return { kind: "unowned", entries: [] };
  }

  const entries = fs.readdirSync(workspace);
  if (entries.length === 0) return { kind: "empty", entries };
  if (!entries.includes(gpt56WorkspaceMarkerName)) {
    return { kind: "unowned", entries };
  }

  const marker = path.join(workspace, gpt56WorkspaceMarkerName);
  const markerStat = fs.lstatSync(marker);
  if (
    !markerStat.isFile() ||
    fs.readFileSync(marker, "utf8") !== gpt56WorkspaceMarkerContents
  ) {
    return { kind: "unowned", entries };
  }

  return { kind: "owned", entries };
}

export function assertPreparedGpt56Workspace(
  workspace,
  { label = "GPT-5.6 benchmark workspace" } = {},
) {
  const resolvedWorkspace = assertSafeGpt56WorkspaceLocation(workspace, {
    label,
  });
  const state = inspectGpt56Workspace(resolvedWorkspace);
  if (
    state.kind !== "owned" ||
    state.entries.length !== 1 ||
    state.entries[0] !== gpt56WorkspaceMarkerName
  ) {
    throw workspaceError(
      label,
      `must be an existing marker-owned empty workspace containing only ${gpt56WorkspaceMarkerName}`,
    );
  }

  return resolvedWorkspace;
}
