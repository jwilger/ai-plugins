import type {
  ExtensionCommandContext,
  SessionEntry,
} from "@earendil-works/pi-coding-agent";
import { SessionManager } from "@earendil-works/pi-coding-agent";
import { realpathSync, writeFileSync } from "node:fs";

export type WorktreeSessionSwitchResult =
  | Readonly<{
      status: "switched";
      target: string;
      sessionPath: string;
    }>
  | Readonly<{
      status: "cancelled" | "failed";
      target: string;
      sessionPath: string;
      code: string;
    }>
  | Readonly<{
      status: "unsupported";
      target: string;
      code: "development_system.worktree_switch_requires_local_tui";
    }>;

function sanitizeDiagnostic(value: string): string {
  return value
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "")
    .replace(
      /\u001b(?:\[[0-?]*[ -/]*[@-~]|\][^\u0007]*(?:\u0007|\u001b\\))/g,
      "",
    )
    .slice(0, 500);
}

export function prepareWorktreeSession(
  source: ExtensionCommandContext["sessionManager"],
  requestedTarget: string,
): string {
  const target = realpathSync(requestedTarget);
  const sourceFile = source.getSessionFile();
  const entries = source.getBranch() as readonly SessionEntry[];
  const expectedLeaf = source.getLeafId();
  const prepared = SessionManager.create(target, undefined, {
    parentSession: sourceFile,
  });
  const sessionPath = prepared.getSessionFile();
  const header = prepared.getHeader();
  if (!sessionPath || !header)
    throw new Error("development_system.worktree_session_prepare_failed");

  const document = [header, ...entries]
    .map((entry) => JSON.stringify(entry))
    .join("\n");
  writeFileSync(sessionPath, `${document}\n`, {
    encoding: "utf8",
    flag: "wx",
    mode: 0o600,
  });

  const verified = SessionManager.open(sessionPath);
  if (verified.getCwd() !== target || verified.getLeafId() !== expectedLeaf)
    throw new Error("development_system.worktree_session_verification_failed");
  return sessionPath;
}

export async function switchWorktreeSession(
  context: Pick<
    ExtensionCommandContext,
    "cwd" | "mode" | "hasUI" | "sessionManager" | "switchSession" | "ui"
  >,
  requestedTarget: string,
): Promise<WorktreeSessionSwitchResult> {
  const target = realpathSync(requestedTarget);
  if (context.mode !== "tui" || !context.hasUI)
    return {
      status: "unsupported",
      code: "development_system.worktree_switch_requires_local_tui",
      target,
    };

  let sessionPath: string;
  try {
    sessionPath = prepareWorktreeSession(context.sessionManager, target);
  } catch (error) {
    const reason = sanitizeDiagnostic(
      error instanceof Error ? error.message : String(error),
    );
    context.ui.notify(
      `Could not prepare the target Pi session. The worktree was preserved. ${reason}`,
      "error",
    );
    return {
      status: "failed",
      target,
      sessionPath: "",
      code: "development_system.worktree_session_prepare_failed",
    };
  }

  try {
    const switched = await context.switchSession(sessionPath, {
      withSession: async (replacement) => {
        const replacementTarget = realpathSync(replacement.cwd);
        if (replacementTarget !== target)
          throw new Error(
            "development_system.worktree_replacement_cwd_mismatch",
          );
        replacement.ui.notify(
          `Development-system workspace switched to ${sanitizeDiagnostic(target)}. The conversation and active session branch were preserved.`,
          "info",
        );
      },
    });
    if (switched.cancelled) {
      context.ui.notify(
        `Workspace switch cancelled. The prepared private session was retained at ${sanitizeDiagnostic(sessionPath)} for recovery.`,
        "info",
      );
      return {
        status: "cancelled",
        target,
        sessionPath,
        code: "development_system.worktree_switch_cancelled",
      };
    }
    // The source context is stale after successful replacement. Do not access it.
    return { status: "switched", target, sessionPath };
  } catch (error) {
    const reason = sanitizeDiagnostic(
      error instanceof Error ? error.message : String(error),
    );
    context.ui.notify(
      `Workspace switch failed; the worktree and prepared private session were retained. ${reason}`,
      "error",
    );
    return {
      status: "failed",
      target,
      sessionPath,
      code: "development_system.worktree_switch_failed",
    };
  }
}
