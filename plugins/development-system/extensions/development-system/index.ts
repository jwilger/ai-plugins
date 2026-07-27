import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { chmod, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { resolveStatus } from "./adapters/status-interpreter.ts";
import { applySetupPreview, createSetupPreview } from "./adapters/setup.ts";
import type { HarnessMode } from "./core/status.ts";
import { parseProjectPolicy } from "./core/configuration.ts";
import {
  classifyPath,
  classifyShellCommand,
  deliveryDecision,
  guardMessage,
} from "./core/guards.ts";

const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

function concise(status: Awaited<ReturnType<typeof resolveStatus>>): string {
  const summary = status.configured
    ? `delivery=${status.deliveryMode} checkout=${status.checkout.kind} features=${status.enabledFeatures.join(",") || "none"}`
    : `configuration_missing checkout=${status.checkout.kind}`;
  const errors = status.errors.map((error) => error.code).join(",");
  return errors ? `${summary} errors=${errors}` : summary;
}

function pathRejection(kind: string): string {
  const missing =
    kind === "outside"
      ? "a path inside the active checkout"
      : "an extension-owned narrow capability";
  return guardMessage({
    code: `development_system.path_${kind.replaceAll("-", "_")}_blocked`,
    boundary: kind,
    missing,
    nextAction:
      "Use an allowed linked-worktree path and never expose protected repository or secret data.",
  });
}

async function guardContext(cwd: string, mode: HarnessMode) {
  const status = await resolveStatus(cwd, packageRoot, mode);
  let policy = null;
  try {
    policy = parseProjectPolicy(
      await readFile(path.join(cwd, ".development-system.toml"), "utf8"),
    );
  } catch {
    // Missing/invalid policy remains null and cannot grant delivery authority.
  }
  const branchResult = await new Promise<string>((resolve) => {
    import("node:child_process").then(({ execFile }) => {
      execFile(
        "git",
        ["-C", cwd, "branch", "--show-current"],
        (_error, stdout) => resolve(stdout.trim()),
      );
    });
  });
  return { status, policy, branch: branchResult };
}

async function shellRejection(
  command: string,
  cwd: string,
  mode: HarnessMode,
  confirm: (title: string, message: string) => Promise<boolean>,
): Promise<string | null> {
  const classification = classifyShellCommand(command);
  const context = await guardContext(cwd, mode);
  if (
    classification.kind === "delivery" ||
    classification.kind === "destructive-delivery"
  ) {
    const rejection = deliveryDecision({
      mode: context.policy?.delivery.mode ?? null,
      branch: context.branch,
      trunk: context.policy?.delivery.trunkBranch ?? "",
      destructive: classification.kind === "destructive-delivery",
    });
    if (
      rejection?.code === "development_system.destructive_approval_required" &&
      mode === "tui"
    ) {
      const approved = await confirm(
        "Approve destructive delivery operation?",
        command,
      );
      return approved ? null : guardMessage(rejection);
    }
    return rejection ? guardMessage(rejection) : null;
  }
  if (
    context.policy?.features.worktrees &&
    context.status.checkout.kind === "primary" &&
    classification.kind !== "read-only"
  ) {
    return guardMessage({
      code: "development_system.coordination_shell_blocked",
      boundary: "coordination-checkout mutation",
      missing: "a provably read-only bounded command",
      nextAction: "Run ordinary mutation work from a linked worktree.",
    });
  }
  return null;
}

async function recordProvenanceMarker(): Promise<void> {
  const marker = process.env.DEVELOPMENT_SYSTEM_PI_EVAL_MARKER;
  if (!marker) return;
  if (!path.isAbsolute(marker))
    throw new Error("development_system.eval_marker_requires_absolute_path");
  await writeFile(
    marker,
    `${JSON.stringify({
      package: "development-system",
      extension: fileURLToPath(import.meta.url),
      version: "1.2.0",
    })}\n`,
    { mode: 0o600 },
  );
  await chmod(marker, 0o600);
}

/** Pi adapter composition root. Domain behavior lives in pure core modules. */
export default function developmentSystemExtension(pi: ExtensionAPI): void {
  let started = false;

  pi.registerCommand("development-system-status", {
    description: "Show deterministic development-system project status",
    handler: async (_arguments, context) => {
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      context.ui.notify(
        JSON.stringify(status, null, 2),
        status.errors.length > 0 ? "warning" : "info",
      );
    },
  });

  pi.registerCommand("development-system-setup", {
    description:
      "Preview setup and require bound local-TUI confirmation before applying",
    handler: async (arguments_, context) => {
      try {
        const preview = await createSetupPreview(
          packageRoot,
          context.cwd,
          arguments_,
        );
        context.ui.notify(
          `${preview.preview}approval_binding ${preview.binding}`,
          "info",
        );
        if (context.mode !== "tui") {
          context.ui.notify(
            "development_system.setup_confirmation_required: rerun this command in the local Pi TUI; RPC, print, and JSON responses cannot approve mutation.",
            "warning",
          );
          return;
        }
        const confirmed = await context.ui.confirm(
          "Apply this exact development-system setup?",
          `${preview.preview}\nBinding: ${preview.binding}`,
        );
        if (!confirmed) return;
        const result = await applySetupPreview(packageRoot, preview);
        context.ui.notify(result, "info");
      } catch (error) {
        context.ui.notify(
          error instanceof Error ? error.message : String(error),
          "error",
        );
      }
    },
  });

  pi.registerTool({
    name: "development_system_status",
    label: "Development System Status",
    description:
      "Return bounded project workflow, checkout, component, and enforcement status. This tool is read-only and never grants mutation authority.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      return {
        content: [{ type: "text", text: concise(status) }],
        details: status,
      };
    },
  });

  pi.registerTool({
    name: "development_system_setup_preview",
    label: "Development System Setup Preview",
    description:
      "Preview repository setup without mutation. Applying setup is only available through the local-TUI command and its bound confirmation.",
    parameters: {
      type: "object",
      properties: {
        arguments: {
          type: "string",
          description:
            "Optional setup pairs: --preset, --delivery, --enable, or --disable.",
        },
      },
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const preview = await createSetupPreview(
        packageRoot,
        context.cwd,
        typeof parameters.arguments === "string" ? parameters.arguments : "",
      );
      return {
        content: [
          {
            type: "text",
            text: `${preview.preview}development_system.setup_confirmation_required`,
          },
        ],
        details: preview,
      };
    },
  });

  pi.on("tool_call", async (event, context) => {
    if (["write", "edit", "read"].includes(event.toolName)) {
      const rawPath = (event.input as { path?: unknown }).path;
      if (typeof rawPath === "string") {
        const status = await resolveStatus(
          context.cwd,
          packageRoot,
          context.mode as HarnessMode,
        );
        const classified = classifyPath({
          rawPath,
          cwd: context.cwd,
          boundary: status.checkout.current,
        });
        if (classified.kind !== "inside")
          return { block: true, reason: pathRejection(classified.kind) };
        if (
          ["write", "edit"].includes(event.toolName) &&
          status.checkout.kind === "primary"
        ) {
          let worktrees = false;
          try {
            worktrees = parseProjectPolicy(
              await readFile(
                path.join(context.cwd, ".development-system.toml"),
                "utf8",
              ),
            ).features.worktrees;
          } catch {
            /* missing policy grants nothing but does not impose configured worktree policy */
          }
          if (worktrees)
            return {
              block: true,
              reason: guardMessage({
                code: "development_system.coordination_write_blocked",
                boundary: "coordination checkout",
                missing: "an allowed linked-worktree target",
                nextAction: "Create or enter a linked worktree before editing.",
              }),
            };
        }
      }
    }
    if (event.toolName === "bash") {
      const command = (event.input as { command?: unknown }).command;
      if (typeof command === "string") {
        const reason = await shellRejection(
          command,
          context.cwd,
          context.mode as HarnessMode,
          (title, message) => context.ui.confirm(title, message),
        );
        if (reason) return { block: true, reason };
      }
    }
  });

  pi.on("tool_result", async (event) => {
    if (!["grep", "find", "ls"].includes(event.toolName)) return;
    const secretPattern =
      /(^|[/\\])(?:\.env(?:\.[^/\\\s]+)?|credentials(?:\.json)?|auth\.json|id_(?:rsa|ed25519))(?:$|[/\\:\s])/i;
    const content = event.content.map((item) =>
      item.type === "text"
        ? {
            ...item,
            text: item.text
              .split("\n")
              .filter((line) => !secretPattern.test(line))
              .join("\n"),
          }
        : item,
    );
    return { content };
  });

  pi.on("user_bash", async (event, context) => {
    const reason = await shellRejection(
      event.command,
      event.cwd,
      context.mode as HarnessMode,
      (title, message) => context.ui.confirm(title, message),
    );
    if (!reason) return;
    return {
      result: {
        output: reason,
        exitCode: 2,
        cancelled: false,
        truncated: false,
      },
    };
  });

  pi.on("session_start", async (_event, context) => {
    started = true;
    await recordProvenanceMarker();
    const status = await resolveStatus(
      context.cwd,
      packageRoot,
      context.mode as HarnessMode,
    );
    context.ui.setStatus(
      "development-system",
      status.configured
        ? `development-system: ${status.deliveryMode}`
        : "development-system: setup required",
    );
    for (const error of status.errors)
      context.ui.notify(`${error.code}: ${error.nextAction}`, "warning");
    for (const diagnostic of status.diagnostics)
      context.ui.notify(diagnostic, "warning");
    const ownedOrMediated = new Set([
      "read",
      "write",
      "edit",
      "bash",
      "grep",
      "find",
      "ls",
      "development_system_status",
      "development_system_setup_preview",
    ]);
    const unknownBoundaryTools =
      pi
        .getAllTools?.()
        .filter(
          (tool) =>
            !ownedOrMediated.has(tool.name) &&
            tool.sourceInfo?.source !== "builtin",
        ) ?? [];
    if (unknownBoundaryTools.length > 0) {
      context.ui.notify(
        `development_system.guarded_composition_limited unknown_tools=${unknownBoundaryTools.map((tool) => tool.name).join(",")}`,
        "warning",
      );
    }
  });

  pi.on("session_shutdown", async (_event, context) => {
    if (!started) return;
    started = false;
    context.ui.setStatus("development-system", undefined);
  });
}
