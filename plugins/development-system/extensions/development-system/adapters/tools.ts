import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";

const execFileAsync = promisify(execFile);

export type ManagedToolStatus = Readonly<{
  name: string;
  requiredFor: readonly string[];
  targetVersion: string;
  status: "missing" | "invalid" | "outdated" | "compatible";
  currentVersion: string | null;
  executable: string | null;
  source: "path" | "user-global" | null;
}>;

export type ManagedToolPolicy = Readonly<{
  schemaVersion: 1;
  destination: string;
  installationScope: "user-global";
  requiresSudo: false;
  inheritedPathIncludesDestination: boolean;
  pathAction: string | null;
  tools: readonly ManagedToolStatus[];
  installed?: readonly string[];
}>;

function parsePolicy(output: string): ManagedToolPolicy {
  let value: unknown;
  try {
    value = JSON.parse(output);
  } catch {
    throw new Error("development_system.tool_status_invalid");
  }
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("development_system.tool_status_invalid");
  const policy = value as Partial<ManagedToolPolicy>;
  if (
    policy.schemaVersion !== 1 ||
    typeof policy.destination !== "string" ||
    policy.installationScope !== "user-global" ||
    policy.requiresSudo !== false ||
    typeof policy.inheritedPathIncludesDestination !== "boolean" ||
    !Array.isArray(policy.tools)
  )
    throw new Error("development_system.tool_status_invalid");
  for (const tool of policy.tools) {
    if (
      !tool ||
      typeof tool.name !== "string" ||
      !Array.isArray(tool.requiredFor) ||
      typeof tool.targetVersion !== "string" ||
      !["missing", "invalid", "outdated", "compatible"].includes(tool.status)
    )
      throw new Error("development_system.tool_status_invalid");
  }
  return Object.freeze({
    ...policy,
    pathAction: policy.pathAction ?? null,
    tools: Object.freeze([...policy.tools]),
    installed: policy.installed
      ? Object.freeze([...policy.installed])
      : undefined,
  } as ManagedToolPolicy);
}

async function runInstaller(
  packageRoot: string,
  arguments_: readonly string[],
): Promise<ManagedToolPolicy> {
  const { stdout } = await execFileAsync(
    process.execPath,
    [path.join(packageRoot, "bin/install-development-tool.mjs"), ...arguments_],
    {
      env: process.env,
      timeout: 180_000,
      maxBuffer: 1024 * 1024,
    },
  );
  return parsePolicy(stdout);
}

function selectedPolicy(
  policy: ManagedToolPolicy,
  requiredFeatures: readonly string[],
): ManagedToolPolicy {
  return Object.freeze({
    ...policy,
    tools: Object.freeze(
      policy.tools.filter((tool) =>
        tool.requiredFor.some((feature) => requiredFeatures.includes(feature)),
      ),
    ),
  });
}

export async function inspectManagedTools(
  packageRoot: string,
  requiredFeatures: readonly string[],
): Promise<ManagedToolPolicy> {
  return selectedPolicy(
    await runInstaller(packageRoot, ["status", "--json"]),
    requiredFeatures,
  );
}

function prependProcessPath(directory: string): void {
  const entries = (process.env.PATH ?? "").split(path.delimiter);
  if (!entries.some((entry) => path.resolve(entry || ".") === directory))
    process.env.PATH = `${directory}${path.delimiter}${process.env.PATH ?? ""}`;
}

export async function reconcileManagedTools(
  packageRoot: string,
  requiredFeatures: readonly string[],
): Promise<ManagedToolPolicy> {
  const before = await inspectManagedTools(packageRoot, requiredFeatures);
  const incompatible = before.tools.filter(
    (tool) => tool.status !== "compatible",
  );
  let result = before;
  if (incompatible.length > 0) {
    result = selectedPolicy(
      await runInstaller(packageRoot, [
        "install",
        "--json",
        ...incompatible.map((tool) => tool.name),
      ]),
      requiredFeatures,
    );
    prependProcessPath(result.destination);
  } else if (result.tools.some((tool) => tool.source === "user-global")) {
    prependProcessPath(result.destination);
  }
  return result;
}

export function managedToolOffer(policy: ManagedToolPolicy): string {
  const lines = policy.tools.map((tool) => {
    const current = tool.currentVersion ?? tool.status;
    return `${tool.name}: current=${current} status=${tool.status} target=${tool.targetVersion}`;
  });
  return [
    ...lines,
    `destination: ${policy.destination}`,
    "scope: user-global for the current user across projects",
    "sudo: not required",
  ].join("\n");
}

export function managedToolResult(policy: ManagedToolPolicy): string {
  const versions = policy.tools
    .map((tool) => `${tool.name}=${tool.targetVersion}`)
    .join(" ");
  const installed = policy.installed ?? [];
  const outcome =
    installed.length > 0
      ? `development_system.setup_tools_installed ${versions} destination=${policy.destination}`
      : `development_system.setup_tools_compatible ${versions}`;
  const usesUserGlobal =
    installed.length > 0 ||
    policy.tools.some((tool) => tool.source === "user-global");
  return !usesUserGlobal ||
    policy.inheritedPathIncludesDestination ||
    !policy.pathAction
    ? outcome
    : `${outcome}\ndevelopment_system.user_global_bin_not_in_inherited_path action=${JSON.stringify(policy.pathAction)}`;
}
