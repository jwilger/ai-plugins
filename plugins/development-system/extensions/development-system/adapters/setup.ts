import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";
import { lstat, realpath } from "node:fs/promises";

const execFileAsync = promisify(execFile);

export type SetupPreview = Readonly<{
  project: string;
  primaryCheckout: string;
  arguments: readonly string[];
  preview: string;
  binding: string;
}>;

function safeArguments(raw: string): string[] {
  const values = raw.trim() ? raw.trim().split(/\s+/) : [];
  const allowedOptions = new Set([
    "--preset",
    "--delivery",
    "--enable",
    "--disable",
  ]);
  const output: string[] = [];
  for (let index = 0; index < values.length; index += 2) {
    const option = values[index];
    const value = values[index + 1];
    if (!allowedOptions.has(option) || !value || value.startsWith("--")) {
      throw new Error(
        `development_system.setup_arguments_invalid option=${option ?? "missing"}`,
      );
    }
    output.push(option, value);
  }
  return output;
}

async function preconditions(project: string): Promise<
  Readonly<{
    project: string;
    primary: string;
    head: string;
    trackedStatus: string;
    configIdentity: string | null;
  }>
> {
  const canonicalProject = await realpath(project);
  const [
    { stdout: gitDirectory },
    { stdout: commonDirectory },
    { stdout: head },
    { stdout: trackedStatus },
  ] = await Promise.all([
    execFileAsync("git", [
      "-C",
      canonicalProject,
      "rev-parse",
      "--path-format=absolute",
      "--git-dir",
    ]),
    execFileAsync("git", [
      "-C",
      canonicalProject,
      "rev-parse",
      "--path-format=absolute",
      "--git-common-dir",
    ]),
    execFileAsync("git", ["-C", canonicalProject, "rev-parse", "HEAD"]),
    execFileAsync("git", [
      "-C",
      canonicalProject,
      "status",
      "--porcelain=v1",
      "--untracked-files=no",
    ]),
  ]);
  const git = await realpath(gitDirectory.trim());
  const common = await realpath(commonDirectory.trim());
  const primary = path.dirname(common);
  if (git !== common)
    throw new Error(
      `development_system.setup_requires_primary_checkout primary_checkout=${primary}`,
    );
  let configIdentity: string | null = null;
  try {
    const stat = await lstat(
      path.join(canonicalProject, ".development-system.toml"),
    );
    configIdentity = `${stat.dev}:${stat.ino}:${stat.size}:${stat.mtimeMs}:${stat.isSymbolicLink()}`;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  return {
    project: canonicalProject,
    primary,
    head: head.trim(),
    trackedStatus,
    configIdentity,
  };
}

function bind(value: unknown): string {
  return createHash("sha256").update(JSON.stringify(value)).digest("hex");
}

export async function createSetupPreview(
  packageRoot: string,
  project: string,
  rawArguments: string,
): Promise<SetupPreview> {
  const arguments_ = safeArguments(rawArguments);
  const state = await preconditions(project);
  const { stdout: preview } = await execFileAsync(
    path.join(packageRoot, "bin/development-system"),
    ["setup", "--project", state.project, ...arguments_, "--dry-run"],
  );
  return Object.freeze({
    project: state.project,
    primaryCheckout: state.primary,
    arguments: arguments_,
    preview,
    binding: bind({ state, arguments_, preview }),
  });
}

export async function applySetupPreview(
  packageRoot: string,
  approved: SetupPreview,
): Promise<string> {
  const current = await createSetupPreview(
    packageRoot,
    approved.project,
    approved.arguments.join(" "),
  );
  if (current.binding !== approved.binding)
    throw new Error("development_system.setup_confirmation_stale");
  const { stdout } = await execFileAsync(
    path.join(packageRoot, "bin/development-system"),
    [
      "setup",
      "--project",
      approved.project,
      ...approved.arguments,
      "--apply",
      "--yes",
    ],
  );
  return stdout;
}
