import { createHash, randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";
import {
  lstat,
  readFile,
  realpath,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
import { parseProjectPolicy } from "../core/configuration.ts";

const execFileAsync = promisify(execFile);

export type SetupPreview = Readonly<{
  project: string;
  primaryCheckout: string;
  arguments: readonly string[];
  preview: string;
  binding: string;
  existingConfig: boolean;
  proposedConfig: string | null;
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
    configSource: string | null;
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
  let configSource: string | null = null;
  try {
    const configPath = path.join(canonicalProject, ".development-system.toml");
    const stat = await lstat(configPath);
    if (stat.isSymbolicLink())
      throw new Error("development_system.setup_config_symlink_blocked");
    configIdentity = `${stat.dev}:${stat.ino}:${stat.size}:${stat.mtimeMs}:false`;
    configSource = await readFile(configPath, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  return {
    project: canonicalProject,
    primary,
    head: head.trim(),
    trackedStatus,
    configIdentity,
    configSource,
  };
}

function bind(value: unknown): string {
  return createHash("sha256").update(JSON.stringify(value)).digest("hex");
}

function replaceAssignment(
  source: string,
  sectionName: string,
  keyName: string,
  value: string,
): string {
  let section = "";
  let replaced = false;
  const lines = source.split("\n").map((line) => {
    const sectionMatch = line.trim().match(/^\[([a-z0-9_.-]+)]$/);
    if (sectionMatch) section = sectionMatch[1];
    if (section !== sectionName) return line;
    const assignment = line.match(
      new RegExp(`^(\\s*${keyName}\\s*=\\s*)([^#]*?)(\\s*(?:#.*)?)$`),
    );
    if (!assignment) return line;
    if (replaced)
      throw new Error(
        `development_system.setup_duplicate_assignment field=${sectionName}.${keyName}`,
      );
    replaced = true;
    return `${assignment[1]}${value}${assignment[3]}`;
  });
  if (!replaced)
    throw new Error(
      `development_system.setup_assignment_missing field=${sectionName}.${keyName}`,
    );
  return lines.join("\n");
}

function proposeExistingConfig(source: string, arguments_: readonly string[]) {
  let proposed = source;
  const featureKeys: Record<string, string> = {
    worktrees: "worktrees",
    tiber: "tiber",
    "agentic-systems": "agentic_systems",
    "eval-case-reporting": "eval_case_reporting",
  };
  for (let index = 0; index < arguments_.length; index += 2) {
    const option = arguments_[index];
    const value = arguments_[index + 1];
    if (option === "--preset" && value !== "personal-trunk")
      throw new Error(`development_system.unsupported_preset preset=${value}`);
    if (option === "--delivery")
      proposed = replaceAssignment(
        proposed,
        "delivery",
        "mode",
        JSON.stringify(value),
      );
    if (option === "--enable" || option === "--disable") {
      const key = featureKeys[value];
      if (!key)
        throw new Error(`development_system.unknown_feature feature=${value}`);
      proposed = replaceAssignment(
        proposed,
        "features",
        key,
        option === "--enable" ? "true" : "false",
      );
    }
  }
  const policy = parseProjectPolicy(proposed);
  return { proposed, policy };
}

function existingPreview(
  project: string,
  source: string,
  arguments_: string[],
) {
  const { proposed, policy } = proposeExistingConfig(source, arguments_);
  const preview = [
    `development_system.setup_preview project=${project}`,
    `delivery ${policy.delivery.mode}`,
    `features worktrees=${policy.features.worktrees} tiber=${policy.features.tiber} agentic_systems=${policy.features.agenticSystems} eval_case_reporting=${policy.features.evalCaseReporting}`,
    "preserve unspecified existing configuration",
    "update .development-system.toml",
    "",
  ].join("\n");
  return { proposed, preview };
}

async function atomicWrite(file: string, source: string): Promise<void> {
  const temporary = `${file}.tmp.${process.pid}.${randomUUID()}`;
  try {
    await writeFile(temporary, source, { flag: "wx", mode: 0o600 });
    await rename(temporary, file);
  } finally {
    await rm(temporary, { force: true });
  }
}

export async function createSetupPreview(
  packageRoot: string,
  project: string,
  rawArguments: string,
): Promise<SetupPreview> {
  const arguments_ = safeArguments(rawArguments);
  const state = await preconditions(project);
  let preview: string;
  let proposedConfig: string | null = null;
  if (state.configSource === null) {
    const result = await execFileAsync(
      path.join(packageRoot, "bin/development-system"),
      ["setup", "--project", state.project, ...arguments_, "--dry-run"],
    );
    preview = result.stdout;
  } else {
    const existing = existingPreview(
      state.project,
      state.configSource,
      arguments_,
    );
    preview = existing.preview;
    proposedConfig = existing.proposed;
  }
  return Object.freeze({
    project: state.project,
    primaryCheckout: state.primary,
    arguments: arguments_,
    preview,
    binding: bind({ state, arguments_, preview, proposedConfig }),
    existingConfig: state.configSource !== null,
    proposedConfig,
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
  if (!approved.existingConfig) {
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
  if (approved.proposedConfig === null)
    throw new Error("development_system.setup_proposal_missing");
  const state = await preconditions(approved.project);
  if (state.trackedStatus)
    throw new Error("development_system.setup_tracked_changes_present");
  if (state.configSource === approved.proposedConfig)
    return "development_system.setup_no_changes\n";
  if (state.configSource === null)
    throw new Error("development_system.setup_confirmation_stale");
  const configPath = path.join(approved.project, ".development-system.toml");
  const original = state.configSource;
  await atomicWrite(configPath, approved.proposedConfig);
  try {
    await execFileAsync("git", [
      "-C",
      approved.project,
      "add",
      "--",
      ".development-system.toml",
    ]);
    await execFileAsync("git", [
      "-C",
      approved.project,
      "commit",
      "-m",
      "chore: update development system",
      "-m",
      "Preserve existing project policy while applying explicitly requested setup changes.",
    ]);
  } catch {
    await atomicWrite(configPath, original);
    await execFileAsync("git", [
      "-C",
      approved.project,
      "reset",
      "-q",
      "HEAD",
      "--",
      ".development-system.toml",
    ]);
    throw new Error("development_system.setup_commit_failed");
  }
  const { stdout: head } = await execFileAsync("git", [
    "-C",
    approved.project,
    "rev-parse",
    "HEAD",
  ]);
  return `development_system.setup_applied commit=${head.trim()}\n`;
}
