import { execFile } from "node:child_process";
import { lstat, mkdir, realpath, readFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { parseProjectPolicy } from "../core/configuration.ts";
import {
  configuredWorktreeRoot,
  parseWorktreeBranch,
  parseWorktreeName,
  relaunchCommand,
} from "../core/worktrees.ts";

const execFileAsync = promisify(execFile);
const repositoryQueues = new Map<string, Promise<void>>();

export type WorktreeRecord = Readonly<{
  path: string;
  branch: string | null;
  head: string;
  current: boolean;
  primary: boolean;
  relaunchCommand: string;
  switchCommand: string;
}>;

export type WorktreeInventory = Readonly<{
  primary: string;
  current: string;
  currentKind: "primary" | "linked";
  configuredRoot: string;
  worktrees: readonly WorktreeRecord[];
  requiresRelaunch: false;
  requiresUserWorkspaceSwitch: boolean;
}>;

export type WorktreeCreationResult =
  | Readonly<{
      status: "created";
      path: string;
      branch: string;
      head: string;
      relaunchCommand: string;
      switchCommand: string;
      requiresRelaunch: false;
      requiresUserWorkspaceSwitch: true;
      currentSessionCheckout: string;
      nextAction: string;
    }>
  | Readonly<{
      status: "collision" | "failed";
      code: string;
      path: string;
      branch: string;
      relaunchCommand?: string;
      switchCommand?: string;
      requiresRelaunch: false;
      requiresUserWorkspaceSwitch: true;
      nextAction: string;
    }>;

type RepositoryContext = Readonly<{
  primary: string;
  current: string;
  currentKind: "primary" | "linked";
  configuredRoot: string;
}>;

function isWithin(boundary: string, candidate: string): boolean {
  const relative = path.relative(boundary, candidate);
  return (
    relative !== "" &&
    relative !== ".." &&
    !relative.startsWith(`..${path.sep}`) &&
    !path.isAbsolute(relative)
  );
}

async function canonicalProspective(candidate: string): Promise<string> {
  const missing: string[] = [];
  let ancestor = candidate;
  for (;;) {
    try {
      return path.join(await realpath(ancestor), ...missing);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
      const parent = path.dirname(ancestor);
      if (parent === ancestor) throw error;
      missing.unshift(path.basename(ancestor));
      ancestor = parent;
    }
  }
}

async function repositoryContext(cwd: string): Promise<RepositoryContext> {
  const current = await realpath(cwd);
  const [{ stdout: gitDirectory }, { stdout: commonDirectory }] =
    await Promise.all([
      execFileAsync("git", [
        "-C",
        current,
        "rev-parse",
        "--path-format=absolute",
        "--git-dir",
      ]),
      execFileAsync("git", [
        "-C",
        current,
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
      ]),
    ]);
  const git = await realpath(gitDirectory.trim());
  const common = await realpath(commonDirectory.trim());
  const primary = path.dirname(common);
  const policy = parseProjectPolicy(
    await readFile(path.join(primary, ".development-system.toml"), "utf8"),
  );
  if (!policy.features.worktrees)
    throw new Error("development_system.worktrees_not_enabled");
  const lexicalRoot = configuredWorktreeRoot(primary, policy.worktrees.root);
  const [canonicalPrimary, configuredRoot] = await Promise.all([
    realpath(primary),
    canonicalProspective(lexicalRoot),
  ]);
  if (!isWithin(canonicalPrimary, configuredRoot))
    throw new Error("development_system.worktree_root_symlink_escape");
  return {
    primary: canonicalPrimary,
    current,
    currentKind: git === common ? "primary" : "linked",
    configuredRoot,
  };
}

function parsePorcelain(
  source: string,
  context: RepositoryContext,
): WorktreeRecord[] {
  const records: WorktreeRecord[] = [];
  let current: { path?: string; branch?: string; head?: string } = {};
  const flush = () => {
    if (!current.path || !current.head) return;
    const worktreePath = current.path;
    records.push({
      path: worktreePath,
      head: current.head,
      branch: current.branch ?? null,
      current: worktreePath === context.current,
      primary: worktreePath === context.primary,
      relaunchCommand: relaunchCommand(worktreePath),
      switchCommand: `/development-system-worktree-switch ${current.branch ?? worktreePath}`,
    });
    current = {};
  };
  for (const field of source.split("\0")) {
    if (!field) {
      flush();
      continue;
    }
    if (field.startsWith("worktree ")) {
      flush();
      current.path = field.slice("worktree ".length);
    } else if (field.startsWith("HEAD ")) {
      current.head = field.slice("HEAD ".length);
    } else if (field.startsWith("branch refs/heads/")) {
      current.branch = field.slice("branch refs/heads/".length);
    }
  }
  flush();
  return records;
}

export async function listWorktrees(cwd: string): Promise<WorktreeInventory> {
  const context = await repositoryContext(cwd);
  const { stdout } = await execFileAsync("git", [
    "-C",
    context.primary,
    "worktree",
    "list",
    "--porcelain",
    "-z",
  ]);
  return {
    ...context,
    worktrees: parsePorcelain(stdout, context),
    requiresRelaunch: false,
    requiresUserWorkspaceSwitch: context.currentKind === "primary",
  };
}

async function exists(candidate: string): Promise<boolean> {
  try {
    await lstat(candidate);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return false;
    throw error;
  }
}

async function branchExists(primary: string, branch: string): Promise<boolean> {
  const { stdout } = await execFileAsync("git", [
    "-C",
    primary,
    "branch",
    "--list",
    branch,
    "--format=%(refname)",
  ]);
  return stdout.trim() === `refs/heads/${branch}`;
}

async function withRepositoryQueue<T>(
  primary: string,
  operation: () => Promise<T>,
): Promise<T> {
  const previous = repositoryQueues.get(primary) ?? Promise.resolve();
  let release = () => {};
  const current = new Promise<void>((resolve) => {
    release = resolve;
  });
  const chain = previous.then(() => current);
  repositoryQueues.set(primary, chain);
  await previous;
  try {
    return await operation();
  } finally {
    release();
    if (repositoryQueues.get(primary) === chain)
      repositoryQueues.delete(primary);
  }
}

export async function createWorktree(
  cwd: string,
  input: Readonly<{ name: unknown; branch: unknown }>,
): Promise<WorktreeCreationResult> {
  const name = parseWorktreeName(input.name);
  const branch = parseWorktreeBranch(input.branch);
  const context = await repositoryContext(cwd);
  if (context.currentKind !== "primary")
    throw new Error(
      `development_system.worktree_creation_requires_primary primary=${context.primary}`,
    );
  const target = await canonicalProspective(
    path.join(context.configuredRoot, name),
  );
  if (!isWithin(context.configuredRoot, target))
    throw new Error("development_system.worktree_target_symlink_escape");

  return withRepositoryQueue(context.primary, async () => {
    await execFileAsync("git", ["check-ref-format", "--branch", branch]);
    const inventory = await listWorktrees(context.primary);
    const existingPath = inventory.worktrees.find(
      (worktree) => worktree.path === target,
    );
    const existingBranch = inventory.worktrees.find(
      (worktree) => worktree.branch === branch,
    );
    if (existingPath || (await exists(target)))
      return {
        status: "collision",
        code: "development_system.worktree_path_exists",
        path: target,
        branch,
        relaunchCommand: existingPath?.relaunchCommand,
        switchCommand: existingPath?.switchCommand,
        requiresRelaunch: false,
        requiresUserWorkspaceSwitch: true,
        nextAction: existingPath
          ? `Switch this Pi conversation with: ${existingPath.switchCommand}`
          : "Choose a different worktree name; the existing path was preserved.",
      };
    if (existingBranch || (await branchExists(context.primary, branch)))
      return {
        status: "collision",
        code: "development_system.worktree_branch_exists",
        path: existingBranch?.path ?? target,
        branch,
        relaunchCommand: existingBranch?.relaunchCommand,
        switchCommand: existingBranch?.switchCommand,
        requiresRelaunch: false,
        requiresUserWorkspaceSwitch: true,
        nextAction: existingBranch
          ? `Switch this Pi conversation with: ${existingBranch.switchCommand}`
          : "Choose a different new branch or attach the existing branch manually after review.",
      };

    await mkdir(context.configuredRoot, { recursive: true });
    const materializedRoot = await realpath(context.configuredRoot);
    if (materializedRoot !== context.configuredRoot)
      throw new Error("development_system.worktree_root_changed");
    try {
      await mkdir(target);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
      const reconciled = await listWorktrees(context.primary);
      const created = reconciled.worktrees.find(
        (worktree) => worktree.path === target || worktree.branch === branch,
      );
      return {
        status: "collision",
        code: "development_system.worktree_concurrent_collision",
        path: created?.path ?? target,
        branch,
        relaunchCommand: created?.relaunchCommand,
        switchCommand: created?.switchCommand,
        requiresRelaunch: false,
        requiresUserWorkspaceSwitch: true,
        nextAction: created
          ? `A concurrent creator won; switch with: ${created.switchCommand}`
          : "The target appeared concurrently and was preserved; list worktrees and choose a different name.",
      };
    }
    const materializedTarget = await realpath(target);
    if (!isWithin(materializedRoot, materializedTarget))
      throw new Error("development_system.worktree_target_symlink_escape");
    try {
      await execFileAsync("git", [
        "-C",
        context.primary,
        "worktree",
        "add",
        target,
        "-b",
        branch,
      ]);
    } catch {
      const reconciled = await listWorktrees(context.primary);
      const created = reconciled.worktrees.find(
        (worktree) => worktree.path === target || worktree.branch === branch,
      );
      return {
        status: created ? "collision" : "failed",
        code: created
          ? "development_system.worktree_concurrent_collision"
          : "development_system.worktree_git_failed",
        path: created?.path ?? target,
        branch,
        relaunchCommand: created?.relaunchCommand,
        switchCommand: created?.switchCommand,
        requiresRelaunch: false,
        requiresUserWorkspaceSwitch: true,
        nextAction: created
          ? `A concurrent creator won; switch with: ${created.switchCommand}`
          : "Run development_system_worktree_list, inspect Git worktree state, and retry with a new name and branch. No cleanup was performed.",
      };
    }
    const created = (await listWorktrees(context.primary)).worktrees.find(
      (worktree) => worktree.path === target,
    );
    if (!created)
      throw new Error("development_system.worktree_creation_unobservable");
    return {
      status: "created",
      path: created.path,
      branch,
      head: created.head,
      relaunchCommand: created.relaunchCommand,
      switchCommand: created.switchCommand,
      requiresRelaunch: false,
      requiresUserWorkspaceSwitch: true,
      currentSessionCheckout: context.current,
      nextAction: `In the local Pi TUI, preserve this conversation and rebuild its cwd-bound runtime with: ${created.switchCommand}. Headless callers can instead start a new process with: ${created.relaunchCommand}`,
    };
  });
}
