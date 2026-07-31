import { execFile } from "node:child_process";
import { readFileSync } from "node:fs";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const releasePolicy = JSON.parse(
  readFileSync(
    new URL("../../../bin/tool-releases.json", import.meta.url),
    "utf8",
  ),
) as { tools: { bd: { version: string } } };
const MINIMUM_VERSION = Object.freeze(
  releasePolicy.tools.bd.version.split(".").map(Number),
);
const CI_RECOVERY_LABEL = "development-system:ci-recovery";

export type BeadsIssue = Readonly<{
  id: string;
  title: string;
  status: string;
  labels: readonly string[];
}>;

export type CiRecoveryHold = Readonly<{ incidentId: string; state: string }>;

export function failClosedCiRecoveryHold(error: unknown): CiRecoveryHold {
  const ambiguous =
    error instanceof Error &&
    error.message.startsWith("development_system.beads_ci_recovery_ambiguous");
  return Object.freeze({
    incidentId: ambiguous
      ? "ambiguous-active-incidents"
      : "coordination-unavailable",
    state: ambiguous ? "ambiguous" : "unavailable",
  });
}

export function beadsMinimumVersion(): string {
  return releasePolicy.tools.bd.version;
}

export function parseBeadsVersion(
  output: string,
): readonly [number, number, number] {
  const match = output.match(/\bbd version (\d+)\.(\d+)\.(\d+)\b/);
  if (!match) throw new Error("development_system.beads_version_invalid");
  const version = [
    Number(match[1]),
    Number(match[2]),
    Number(match[3]),
  ] as const;
  for (let index = 0; index < 3; index += 1) {
    if (version[index] > MINIMUM_VERSION[index]) return version;
    if (version[index] < MINIMUM_VERSION[index])
      throw new Error(
        `development_system.beads_version_unsupported minimum=${MINIMUM_VERSION.join(".")}`,
      );
  }
  return version;
}

export function parseBeadsIssueList(output: string): readonly BeadsIssue[] {
  let value: unknown;
  try {
    value = JSON.parse(output);
  } catch {
    throw new Error("development_system.beads_json_invalid");
  }
  if (
    value &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    Object.hasOwn(value, "data")
  ) {
    const envelope = value as { schema_version?: unknown; data?: unknown };
    if (envelope.schema_version !== 1)
      throw new Error("development_system.beads_schema_unsupported");
    value = envelope.data;
  }
  if (!Array.isArray(value))
    throw new Error("development_system.beads_issue_list_invalid");
  return value.map((item) => {
    if (!item || typeof item !== "object" || Array.isArray(item))
      throw new Error("development_system.beads_issue_invalid");
    const record = item as Record<string, unknown>;
    if (
      typeof record.id !== "string" ||
      typeof record.title !== "string" ||
      typeof record.status !== "string" ||
      !Array.isArray(record.labels) ||
      !record.labels.every((label) => typeof label === "string")
    )
      throw new Error("development_system.beads_issue_invalid");
    return Object.freeze({
      id: record.id,
      title: record.title,
      status: record.status,
      labels: Object.freeze([...record.labels] as string[]),
    });
  });
}

export function selectActiveCiRecovery(
  issues: readonly BeadsIssue[],
): CiRecoveryHold | null {
  const active = issues
    .filter(
      (issue) =>
        issue.status === "in_progress" &&
        issue.labels.includes(CI_RECOVERY_LABEL),
    )
    .sort((left, right) => left.id.localeCompare(right.id));
  if (active.length === 0) return null;
  if (active.length > 1)
    throw new Error(
      `development_system.beads_ci_recovery_ambiguous ids=${active.map((issue) => issue.id).join(",")}`,
    );
  return Object.freeze({ incidentId: active[0].id, state: active[0].status });
}

async function runBd(
  cwd: string,
  args: readonly string[],
  timeout = 10_000,
): Promise<string> {
  const { stdout } = await execFileAsync("bd", [...args], {
    cwd,
    timeout,
    maxBuffer: 256 * 1024,
    env: { ...process.env, BD_JSON_ENVELOPE: "1", BD_NON_INTERACTIVE: "1" },
  });
  return stdout.trim();
}

export async function beadsPrime(cwd: string): Promise<string | null> {
  try {
    parseBeadsVersion(await runBd(cwd, ["version"]));
    return (await runBd(cwd, ["prime", "--full"], 15_000)) || null;
  } catch {
    return null;
  }
}

export async function activeCiRecoveryHold(
  cwd: string,
): Promise<CiRecoveryHold | null> {
  let output: string;
  try {
    output = await runBd(cwd, [
      "list",
      "--status",
      "in_progress",
      "--label",
      CI_RECOVERY_LABEL,
      "--json",
    ]);
  } catch (error) {
    return failClosedCiRecoveryHold(error);
  }
  try {
    return selectActiveCiRecovery(parseBeadsIssueList(output));
  } catch (error) {
    return failClosedCiRecoveryHold(error);
  }
}

export async function beadsAvailability(
  cwd: string,
): Promise<
  Readonly<{ available: boolean; initialized: boolean; error?: string }>
> {
  try {
    parseBeadsVersion(await runBd(cwd, ["version"]));
  } catch (error) {
    return Object.freeze({
      available: false,
      initialized: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
  try {
    await runBd(cwd, ["where"]);
    return Object.freeze({ available: true, initialized: true });
  } catch {
    return Object.freeze({ available: true, initialized: false });
  }
}
