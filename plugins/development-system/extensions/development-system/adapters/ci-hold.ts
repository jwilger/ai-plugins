import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";

const execFileAsync = promisify(execFile);

export type CiRecoveryHold = Readonly<{ incidentId: string; state: string }>;

export function parseCiRecoveryHold(output: string): CiRecoveryHold | null {
  let value: unknown;
  try {
    value = JSON.parse(output);
  } catch {
    return null;
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.incident_id !== "string") return null;
  const releaseProof = record.release_proof;
  if (
    record.state === "resolved" &&
    record.hold_released === true &&
    releaseProof &&
    typeof releaseProof === "object" &&
    !Array.isArray(releaseProof) &&
    (releaseProof as Record<string, unknown>).terminal_status === "success"
  )
    return null;
  return {
    incidentId: record.incident_id,
    state: typeof record.state === "string" ? record.state : "active",
  };
}

export async function activeCiRecoveryHold(
  packageRoot: string,
  cwd: string,
): Promise<CiRecoveryHold | null> {
  try {
    const { stdout } = await execFileAsync(
      path.join(packageRoot, "bin/tiber"),
      ["ci-recovery", "status"],
      { cwd, timeout: 5_000, maxBuffer: 50 * 1024 },
    );
    return parseCiRecoveryHold(stdout);
  } catch {
    return null;
  }
}
