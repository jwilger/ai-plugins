import { createHash, randomUUID } from "node:crypto";
import { constants } from "node:fs";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";
import {
  copyFile,
  lstat,
  mkdir,
  readFile,
  readdir,
  realpath,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
import { parseProjectPolicy } from "../core/configuration.ts";
import {
  inspectManagedTools,
  managedToolOffer,
  managedToolResult,
  reconcileManagedTools,
  type ManagedToolPolicy,
} from "./tools.ts";

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

function deliveryWorkflow(mode: string): string {
  if (mode === "pull-request") return "development-change-pr";
  if (mode === "local-only") return "development-change-local";
  return "development-change-direct";
}

function migrateLegacyTiberPolicy(source: string): string {
  if (!/^schema_version\s*=\s*1\s*$/m.test(source)) return source;
  if (!/^tiber\s*=\s*(?:true|false)\s*$/m.test(source)) return source;
  const delivery =
    source.match(/^mode\s*=\s*"([^"]+)"\s*$/m)?.[1] ?? "direct-to-trunk";
  const lines = source.split("\n");
  const output: string[] = [];
  let section = "";
  for (const line of lines) {
    const sectionMatch = line.trim().match(/^\[([a-z0-9_.-]+)]$/);
    if (sectionMatch) section = sectionMatch[1];
    if (section === "tiber") continue;
    output.push(
      line
        .replace(/^schema_version\s*=\s*1\s*$/, "schema_version = 2")
        .replace(/^(\s*)tiber(\s*=\s*(?:true|false)\s*)$/, "$1beads$2"),
    );
  }
  while (output.at(-1) === "") output.pop();
  output.push("", "[beads]", `workflow = "${deliveryWorkflow(delivery)}"`, "");
  return output.join("\n");
}

function proposeExistingConfig(source: string, arguments_: readonly string[]) {
  let proposed = migrateLegacyTiberPolicy(source);
  const featureKeys: Record<string, string> = {
    worktrees: "worktrees",
    beads: "beads",
    "agentic-systems": "agentic_systems",
    "eval-case-reporting": "eval_case_reporting",
  };
  for (let index = 0; index < arguments_.length; index += 2) {
    const option = arguments_[index];
    const value = arguments_[index + 1];
    if (option === "--preset" && value !== "personal-trunk")
      throw new Error(`development_system.unsupported_preset preset=${value}`);
    if (option === "--delivery") {
      proposed = replaceAssignment(
        proposed,
        "delivery",
        "mode",
        JSON.stringify(value),
      );
      proposed = replaceAssignment(
        proposed,
        "beads",
        "workflow",
        JSON.stringify(deliveryWorkflow(value)),
      );
    }
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

async function existingPreview(
  packageRoot: string,
  project: string,
  source: string,
  arguments_: string[],
) {
  const { proposed, policy } = proposeExistingConfig(source, arguments_);
  const tools = policy.features.beads
    ? await inspectManagedTools(packageRoot, ["beads"])
    : null;
  const preview = [
    `development_system.setup_preview project=${project}`,
    `delivery ${policy.delivery.mode}`,
    `features worktrees=${policy.features.worktrees} beads=${policy.features.beads} agentic_systems=${policy.features.agenticSystems} eval_case_reporting=${policy.features.evalCaseReporting}`,
    ...(tools ? ["required user-global tools:", managedToolOffer(tools)] : []),
    "preserve unspecified existing configuration",
    "reconcile enabled dependencies even when configuration is unchanged",
    "update .development-system.toml only when policy changes",
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
    const existing = await existingPreview(
      packageRoot,
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

async function ensureBeadsWorkspace(
  packageRoot: string,
  project: string,
  bd: string,
): Promise<
  Readonly<{
    created: boolean;
    addedFormulas: readonly string[];
    rollbackAutoCommit: () => Promise<void>;
  }>
> {
  const toolEnvironment = { ...process.env };
  let created = false;
  const addedFormulas: string[] = [];
  let rollbackAutoCommit = async (): Promise<void> => {};
  try {
    const { stdout: initialHeadOutput } = await execFileAsync("git", [
      "-C",
      project,
      "rev-parse",
      "HEAD",
    ]);
    const initialHead = initialHeadOutput.trim();
    try {
      const stat = await lstat(path.join(project, ".beads"));
      if (!stat.isDirectory())
        throw new Error("development_system.beads_workspace_invalid");
      await execFileAsync(bd, ["where"], {
        cwd: project,
        env: toolEnvironment,
        timeout: 5_000,
      });
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
      await execFileAsync(
        bd,
        [
          "init",
          "--quiet",
          "--non-interactive",
          "--skip-agents",
          "--skip-hooks",
        ],
        { cwd: project, env: toolEnvironment, timeout: 30_000 },
      );
      created = true;
      const { stdout: initializedHeadOutput } = await execFileAsync("git", [
        "-C",
        project,
        "rev-parse",
        "HEAD",
      ]);
      const initializedHead = initializedHeadOutput.trim();
      if (initializedHead !== initialHead) {
        const { stdout: parentOutput } = await execFileAsync("git", [
          "-C",
          project,
          "rev-parse",
          `${initializedHead}^`,
        ]);
        if (parentOutput.trim() !== initialHead)
          throw new Error("development_system.beads_init_history_unexpected");
        await execFileAsync("git", [
          "-C",
          project,
          "reset",
          "--soft",
          initialHead,
        ]);
      }
    }
    const sourceDirectory = path.join(packageRoot, "formulas");
    const targetDirectory = path.join(project, ".beads", "formulas");
    await mkdir(targetDirectory, { recursive: true });
    for (const name of (await readdir(sourceDirectory)).filter((entry) =>
      entry.endsWith(".formula.toml"),
    )) {
      const source = path.join(sourceDirectory, name);
      const target = path.join(targetDirectory, name);
      try {
        const [expected, current] = await Promise.all([
          readFile(source),
          readFile(target),
        ]);
        if (!expected.equals(current))
          throw new Error(
            `development_system.beads_formula_conflict path=${target}`,
          );
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        await copyFile(source, target, constants.COPYFILE_EXCL);
        addedFormulas.push(target);
      }
    }
    let previousAutoCommit: string | null = null;
    try {
      const { stdout } = await execFileAsync(
        bd,
        ["config", "get", "dolt.auto-commit"],
        { cwd: project, env: toolEnvironment, timeout: 10_000 },
      );
      previousAutoCommit = stdout.trim();
    } catch {
      // Missing configuration is restored by unsetting the key.
    }
    rollbackAutoCommit = async () => {
      const arguments_ = previousAutoCommit
        ? ["config", "set", "dolt.auto-commit", previousAutoCommit]
        : ["config", "unset", "dolt.auto-commit"];
      await execFileAsync(bd, arguments_, {
        cwd: project,
        env: toolEnvironment,
        timeout: 10_000,
      });
    };
    await execFileAsync(bd, ["config", "set", "dolt.auto-commit", "on"], {
      cwd: project,
      env: toolEnvironment,
      timeout: 10_000,
    });
    return Object.freeze({
      created,
      addedFormulas: Object.freeze(addedFormulas),
      rollbackAutoCommit,
    });
  } catch (error) {
    if (error instanceof Error)
      Object.assign(error, {
        beadsSetup: Object.freeze({
          created,
          addedFormulas: Object.freeze(addedFormulas),
          rollbackAutoCommit,
        }),
      });
    throw error;
  }
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
  if (state.configSource === null)
    throw new Error("development_system.setup_confirmation_stale");
  const policy = parseProjectPolicy(approved.proposedConfig);
  let tools: ManagedToolPolicy | null = null;
  let bd: string | null = null;
  if (policy.features.beads) {
    tools = await reconcileManagedTools(packageRoot, ["beads"]);
    const bdStatus = tools.tools.find((tool) => tool.name === "bd");
    bd = bdStatus?.executable ?? null;
    if (!bd)
      throw new Error(
        `development_system.beads_install_failed minimum=${bdStatus?.targetVersion ?? "unknown"}`,
      );
  }
  const configPath = path.join(approved.project, ".development-system.toml");
  const original = state.configSource;
  const configurationChanged = original !== approved.proposedConfig;
  let beadsSetup: Awaited<ReturnType<typeof ensureBeadsWorkspace>> | null =
    null;
  if (configurationChanged)
    await atomicWrite(configPath, approved.proposedConfig);
  try {
    if (policy.features.beads && bd)
      beadsSetup = await ensureBeadsWorkspace(
        packageRoot,
        approved.project,
        bd,
      );
    const repositoryChanged =
      configurationChanged ||
      Boolean(beadsSetup?.created) ||
      Boolean(beadsSetup?.addedFormulas.length);
    if (repositoryChanged) {
      await execFileAsync("git", [
        "-C",
        approved.project,
        "add",
        "-f",
        "--",
        ...(configurationChanged ? [".development-system.toml"] : []),
        ...(policy.features.beads ? [".beads"] : []),
      ]);
      await execFileAsync("git", [
        "-C",
        approved.project,
        "commit",
        "--no-verify",
        "-m",
        "chore: update development system",
        "-m",
        "Preserve existing project policy while reconciling explicitly enabled capabilities.",
      ]);
    }
    const lines = [
      configurationChanged
        ? "development_system.setup_configuration_updated"
        : "development_system.setup_configuration_unchanged",
    ];
    if (repositoryChanged) {
      const { stdout: head } = await execFileAsync("git", [
        "-C",
        approved.project,
        "rev-parse",
        "HEAD",
      ]);
      lines.push(`development_system.setup_applied commit=${head.trim()}`);
    }
    if (tools) lines.push(managedToolResult(tools));
    return `${lines.join("\n")}\n`;
  } catch (error) {
    const rollbackSetup =
      beadsSetup ??
      (error instanceof Error && "beadsSetup" in error
        ? (
            error as Error & {
              beadsSetup: Awaited<ReturnType<typeof ensureBeadsWorkspace>>;
            }
          ).beadsSetup
        : null);
    try {
      await rollbackSetup?.rollbackAutoCommit();
    } catch {
      // Repository rollback still proceeds when Beads config recovery fails.
    }
    if (configurationChanged) await atomicWrite(configPath, original);
    await execFileAsync("git", [
      "-C",
      approved.project,
      "reset",
      "-q",
      "HEAD",
      "--",
      ".development-system.toml",
      ".beads",
      ".gitignore",
    ]);
    if (rollbackSetup?.created) {
      await rm(path.join(approved.project, ".beads"), {
        recursive: true,
        force: true,
      });
      try {
        await execFileAsync("git", [
          "-C",
          approved.project,
          "checkout",
          "-q",
          "--",
          ".gitignore",
        ]);
      } catch {
        await rm(path.join(approved.project, ".gitignore"), { force: true });
      }
    } else if (rollbackSetup)
      await Promise.all(
        rollbackSetup.addedFormulas.map((file) => rm(file, { force: true })),
      );
    throw new Error("development_system.setup_commit_failed");
  }
}
