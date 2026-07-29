import { execFile, spawn } from "node:child_process";
import { constants } from "node:fs";
import { access, lstat, mkdir, realpath, readFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { parseProjectPolicy } from "../core/configuration.ts";
import {
  configuredWorktreeRoot,
  parseWorktreeBranch,
  parseWorktreeName,
} from "../core/worktrees.ts";

const execFileAsync = promisify(execFile);
const repositoryQueues = new Map<string, Promise<void>>();

export type WorktreeRecord = Readonly<{
  path: string;
  branch: string | null;
  head: string;
  current: boolean;
  primary: boolean;
}>;

export type WorktreeInventory = Readonly<{
  primary: string;
  current: string;
  currentKind: "primary" | "linked";
  configuredRoot: string;
  worktrees: readonly WorktreeRecord[];
}>;

export type WorktreeRemovalResult = Readonly<{
  status: "removed";
  path: string;
  branch: string | null;
  branchPreserved: true;
}>;

export type WorktreeCreationResult =
  | Readonly<{
      status: "created";
      path: string;
      branch: string;
      head: string;
      requiresLogicalWorkspaceActivation: true;
      currentSessionCheckout: string;
      nextAction: string;
    }>
  | Readonly<{
      status: "collision" | "failed";
      code: string;
      path: string;
      branch: string;
      requiresLogicalWorkspaceActivation: true;
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
  const projectPath = await realpath(cwd);
  const [
    { stdout: gitDirectory },
    { stdout: commonDirectory },
    { stdout: topLevel },
  ] = await Promise.all([
    execFileAsync("git", [
      "-C",
      projectPath,
      "rev-parse",
      "--path-format=absolute",
      "--git-dir",
    ]),
    execFileAsync("git", [
      "-C",
      projectPath,
      "rev-parse",
      "--path-format=absolute",
      "--git-common-dir",
    ]),
    execFileAsync("git", ["-C", projectPath, "rev-parse", "--show-toplevel"]),
  ]);
  const git = await realpath(gitDirectory.trim());
  const common = await realpath(commonDirectory.trim());
  const current = await realpath(topLevel.trim());
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

async function ignoredEnvrcIsValuable(worktreePath: string): Promise<boolean> {
  const envrcPath = path.join(worktreePath, ".envrc");
  let metadata;
  try {
    metadata = await lstat(envrcPath);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return false;
    throw error;
  }
  try {
    await execFileAsync("git", [
      "-C",
      worktreePath,
      "check-ignore",
      "--quiet",
      "--",
      ".envrc",
    ]);
  } catch (error) {
    if ((error as { code?: number }).code === 1) return false;
    throw error;
  }
  const generated = "use flake\n";
  if (!metadata.isFile() || metadata.size !== Buffer.byteLength(generated))
    return true;
  return (await readFile(envrcPath, "utf8")) !== generated;
}

function firstValuableIgnoredPath(
  worktreePath: string,
): Promise<string | null> {
  const disposable = [
    ".dependencies",
    ".direnv",
    ".evals",
    "node_modules",
    "target",
    ".env.worktree",
    ".envrc",
  ];
  const args = [
    "-C",
    worktreePath,
    "ls-files",
    "--others",
    "--ignored",
    "--exclude-standard",
    "-z",
    "--",
    ".",
    ...disposable.flatMap((candidate) => [
      `:(exclude)${candidate}`,
      `:(exclude)${candidate}/**`,
    ]),
  ];
  return new Promise((resolve, reject) => {
    const child = spawn("git", args, {
      stdio: ["ignore", "pipe", "pipe"],
    });
    let pending = Buffer.alloc(0);
    let stderr = "";
    let found: string | null = null;
    let outputLimitExceeded = false;
    child.stdout.on("data", (chunk: Buffer) => {
      if (found !== null || outputLimitExceeded) return;
      pending = Buffer.concat([pending, chunk]);
      const separator = pending.indexOf(0);
      if (separator >= 0) {
        found = pending.subarray(0, separator).toString("utf8");
        child.kill("SIGTERM");
        return;
      }
      if (pending.length > 8 * 1024) {
        outputLimitExceeded = true;
        child.kill("SIGTERM");
      }
    });
    child.stderr.on("data", (chunk: Buffer) => {
      if (stderr.length < 8 * 1024)
        stderr += chunk.toString("utf8").slice(0, 8 * 1024 - stderr.length);
    });
    child.once("error", reject);
    child.once("close", (code, signal) => {
      if (found !== null) return resolve(found);
      if (outputLimitExceeded)
        return reject(
          new Error("development_system.worktree_cleanup_ignored_scan_limit"),
        );
      if (code === 0) return resolve(null);
      reject(
        new Error(
          `development_system.worktree_cleanup_ignored_scan_failed code=${code ?? "none"} signal=${signal ?? "none"} stderr=${stderr.replace(/[\\r\\n]+/g, " ").slice(0, 500)}`,
        ),
      );
    });
  });
}

async function assertNoValuableIgnoredState(
  worktreePath: string,
): Promise<void> {
  const envrcValuable = await ignoredEnvrcIsValuable(worktreePath);
  const candidate = envrcValuable
    ? ".envrc"
    : await firstValuableIgnoredPath(worktreePath);
  if (candidate !== null)
    throw new Error(
      `development_system.worktree_cleanup_ignored_state path=${worktreePath} sample=${JSON.stringify(candidate.slice(0, 500))}; transfer or remove ignored state before cleanup`,
    );
}

async function assertCleanWorktree(
  primary: string,
  worktreePath: string,
): Promise<void> {
  const { stdout } = await execFileAsync("git", [
    "-C",
    worktreePath,
    "status",
    "--porcelain",
    "--untracked-files=all",
  ]);
  if (stdout.trim())
    throw new Error(
      `development_system.worktree_cleanup_dirty path=${worktreePath}; commit, transfer, or remove the remaining state before cleanup`,
    );
  const { stdout: registered } = await execFileAsync("git", [
    "-C",
    primary,
    "worktree",
    "list",
    "--porcelain",
    "-z",
  ]);
  if (!registered.includes(`worktree ${worktreePath}\0`))
    throw new Error("development_system.worktree_cleanup_not_registered");
}

export async function validateWorktreeForCleanup(
  cwd: string,
  requestedTarget: string,
): Promise<WorktreeRecord> {
  const context = await repositoryContext(cwd);
  const target = await realpath(requestedTarget);
  if (!isWithin(context.configuredRoot, target))
    throw new Error("development_system.worktree_cleanup_target_outside_root");
  const selected = (await listWorktrees(cwd)).worktrees.find(
    (worktree) => worktree.path === target,
  );
  if (!selected || selected.primary)
    throw new Error("development_system.worktree_cleanup_target_invalid");
  if (selected.branch === null)
    throw new Error("development_system.worktree_cleanup_detached_head");
  await Promise.all([
    assertCleanWorktree(context.primary, target),
    assertNoValuableIgnoredState(target),
  ]);
  return selected;
}

export async function removeWorktree(
  cwd: string,
  expected: WorktreeRecord,
  assertRegistrationIdentity: () => Promise<void> = async () => {},
): Promise<WorktreeRemovalResult> {
  await assertRegistrationIdentity();
  const context = await repositoryContext(cwd);
  const target = await realpath(expected.path);
  if (!isWithin(context.configuredRoot, target))
    throw new Error("development_system.worktree_cleanup_target_outside_root");
  if (expected.primary)
    throw new Error("development_system.worktree_cleanup_target_invalid");
  if (expected.branch === null)
    throw new Error("development_system.worktree_cleanup_detached_head");
  const selected = (await listWorktrees(cwd)).worktrees.find(
    (worktree) => worktree.path === target,
  );
  if (
    !selected ||
    selected.branch !== expected.branch ||
    selected.head !== expected.head
  )
    throw new Error("development_system.worktree_cleanup_identity_changed");
  if (selected.current)
    throw new Error("development_system.worktree_cleanup_switch_required");
  await Promise.all([
    assertCleanWorktree(context.primary, target),
    assertNoValuableIgnoredState(target),
  ]);

  return withRepositoryQueue(context.primary, async () => {
    await assertRegistrationIdentity();
    const revalidated = (await listWorktrees(context.primary)).worktrees.find(
      (worktree) =>
        worktree.path === expected.path &&
        worktree.branch === expected.branch &&
        worktree.head === expected.head,
    );
    if (!revalidated || revalidated.current)
      throw new Error("development_system.worktree_cleanup_identity_changed");
    await Promise.all([
      assertCleanWorktree(context.primary, target),
      assertNoValuableIgnoredState(target),
    ]);

    const teardown = path.join(
      context.primary,
      "scripts",
      "worktree-teardown.sh",
    );
    let teardownPresent = true;
    try {
      await access(teardown, constants.X_OK);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT")
        teardownPresent = false;
      else throw error;
    }
    if (teardownPresent)
      await execFileAsync(teardown, [target], { cwd: context.primary });
    const afterTeardown = (await listWorktrees(context.primary)).worktrees.find(
      (worktree) =>
        worktree.path === expected.path &&
        worktree.branch === expected.branch &&
        worktree.head === expected.head,
    );
    if (!afterTeardown || afterTeardown.current)
      throw new Error("development_system.worktree_cleanup_identity_changed");
    await Promise.all([
      assertCleanWorktree(context.primary, target),
      assertNoValuableIgnoredState(target),
    ]);
    await assertRegistrationIdentity();
    await execFileAsync(
      "git",
      ["-C", context.primary, "worktree", "remove", target],
      { cwd: context.primary },
    );
    const remaining = (await listWorktrees(context.primary)).worktrees.some(
      (worktree) => worktree.path === target,
    );
    if (remaining)
      throw new Error("development_system.worktree_cleanup_unobservable");
    return {
      status: "removed",
      path: target,
      branch: expected.branch,
      branchPreserved: true,
    };
  });
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
        requiresLogicalWorkspaceActivation: true,
        nextAction: existingPath
          ? `Activate the existing worktree with development_system_worktree_switch selector=${JSON.stringify(existingPath.branch ?? existingPath.path)}`
          : "Choose a different worktree name; the existing path was preserved.",
      };
    if (existingBranch || (await branchExists(context.primary, branch)))
      return {
        status: "collision",
        code: "development_system.worktree_branch_exists",
        path: existingBranch?.path ?? target,
        branch,
        requiresLogicalWorkspaceActivation: true,
        nextAction: existingBranch
          ? `Activate the existing worktree with development_system_worktree_switch selector=${JSON.stringify(existingBranch.branch ?? existingBranch.path)}`
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
        requiresLogicalWorkspaceActivation: true,
        nextAction: created
          ? `A concurrent creator won; activate ${created.path} with development_system_worktree_switch.`
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
        requiresLogicalWorkspaceActivation: true,
        nextAction: created
          ? `A concurrent creator won; activate ${created.path} with development_system_worktree_switch.`
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
      requiresLogicalWorkspaceActivation: true,
      currentSessionCheckout: context.current,
      nextAction:
        "Activate the created path as this session's logical workspace; no Pi relaunch is required.",
    };
  });
}
