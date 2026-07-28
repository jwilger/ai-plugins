import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { chmod, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { resolveStatus } from "./adapters/status-interpreter.ts";
import { applySetupPreview, createSetupPreview } from "./adapters/setup.ts";
import { resolveReviewRoute, runReviewChild } from "./adapters/review-child.ts";
import { registerGoalMode } from "./adapters/goal-mode.ts";
import { activeCiRecoveryHold } from "./adapters/ci-hold.ts";
import {
  McpClient,
  publicToolName,
  schemaIsAdmissible,
  type McpTool,
} from "./adapters/mcp-client.ts";
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

async function recordProvenanceMarker(goalCollision: boolean): Promise<void> {
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
      reservedPublicNames: ["goal", "goal_complete", "goal_blocked"],
      goalCollision,
    })}\n`,
    { mode: 0o600 },
  );
  await chmod(marker, 0o600);
}

/** Pi adapter composition root. Domain behavior lives in pure core modules. */
export default function developmentSystemExtension(pi: ExtensionAPI): void {
  let started = false;
  const componentClients: McpClient[] = [];
  const goalMode = registerGoalMode(pi);

  const bridge = async (
    origin: "tiber" | "review",
    command: string,
    args: readonly string[],
    cwd: string,
    approvedNames: ReadonlySet<string>,
  ): Promise<readonly string[]> => {
    const client = new McpClient({ command, args, cwd });
    await client.start();
    componentClients.push(client);
    const tools = (await client.listTools()).filter((tool) =>
      approvedNames.has(tool.name),
    );
    const admitted = tools.filter((tool) =>
      schemaIsAdmissible(tool.inputSchema),
    );
    const existing = new Set(pi.getAllTools().map((tool) => tool.name));
    const registrations = admitted.map((tool) => ({
      tool,
      publicName: publicToolName(origin, tool.name),
    }));
    if (registrations.some(({ publicName }) => existing.has(publicName))) {
      client.stop();
      throw new Error(`development_system.mcp_tool_collision origin=${origin}`);
    }
    for (const { tool, publicName } of registrations) {
      pi.registerTool({
        name: publicName,
        label: `${origin === "tiber" ? "Tiber" : "Final Review"}: ${tool.name}`,
        description:
          tool.description ?? `First-party ${origin} operation ${tool.name}`,
        parameters: tool.inputSchema as McpTool["inputSchema"],
        async execute(_toolCallId, parameters, signal) {
          const result = await client.callTool(tool.name, parameters, signal);
          const text = JSON.stringify(result);
          if (Buffer.byteLength(text) > 50 * 1024) {
            throw new Error("development_system.component_output_limit");
          }
          return { content: [{ type: "text", text }], details: result };
        },
      });
    }
    return registrations.map(({ publicName }) => publicName);
  };

  pi.registerCommand("development-system-status", {
    description: "Show deterministic development-system project status",
    handler: async (_arguments, context) => {
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      context.ui.notify(
        JSON.stringify({ ...status, goal: goalMode.current() }, null, 2),
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
        details: { ...status, goal: goalMode.current() },
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

  pi.registerTool({
    name: "development_system_run_review_assignment",
    label: "Run Fresh Final-Review Assignment",
    description:
      "Run a development-discipline coordinator assignment in a fresh isolated Pi child using the project route for its abstract model role. Failures remain unresolved.",
    parameters: {
      type: "object",
      properties: {
        assignment: { type: "string", minLength: 1 },
        model_role: {
          type: "string",
          enum: [
            "bounded-helper",
            "substantive-worker",
            "strong-reviewer",
            "strong-worker",
          ],
        },
      },
      required: ["assignment", "model_role"],
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, signal, _onUpdate, context) {
      if (
        typeof parameters.assignment !== "string" ||
        typeof parameters.model_role !== "string"
      ) {
        throw new Error("development_system.review_assignment_invalid");
      }
      const policy = parseProjectPolicy(
        await readFile(
          path.join(context.cwd, ".development-system.toml"),
          "utf8",
        ),
      );
      const result = await runReviewChild({
        assignment: {
          assignment: parameters.assignment,
          modelRole: parameters.model_role,
        },
        cwd: context.cwd,
        route: resolveReviewRoute(parameters.model_role, policy.piReviewModels),
        signal,
      });
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        details: result,
      };
    },
  });

  pi.on("tool_call", async (event, context) => {
    if (["write", "edit", "bash"].includes(event.toolName)) {
      const current = await guardContext(
        context.cwd,
        context.mode as HarnessMode,
      );
      if (current.policy?.features.tiber) {
        const hold = await activeCiRecoveryHold(packageRoot, context.cwd);
        if (hold)
          return {
            block: true,
            reason: guardMessage({
              code: "development_system.ci_recovery_hold",
              boundary: "unrelated guarded work",
              missing: `terminal success for CI incident ${hold.incidentId}`,
              nextAction:
                "Complete the authoritative Tiber CI-recovery transition before other guarded work.",
            }),
          };
      }
    }
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

  pi.on("before_agent_start", async (_event, context) => {
    try {
      const policy = parseProjectPolicy(
        await readFile(
          path.join(context.cwd, ".development-system.toml"),
          "utf8",
        ),
      );
      if (!policy.features.tiber) return;
      const hold = await activeCiRecoveryHold(packageRoot, context.cwd);
      if (hold)
        return {
          message: {
            customType: "development-system-ci-hold",
            content: `development_system.ci_recovery_hold incident=${hold.incidentId} state=${hold.state}. Do not claim readiness or start unrelated guarded work; continue authoritative Tiber recovery.`,
            display: true,
          },
        };
    } catch {
      return;
    }
  });

  pi.on("session_start", async (_event, context) => {
    started = true;
    await recordProvenanceMarker(goalMode.collision);
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
      "development_system_run_review_assignment",
      "goal_complete",
      "goal_blocked",
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
    if (status.configured) {
      try {
        const reviewTools = await bridge(
          "review",
          path.join(packageRoot, "bin/development-discipline-mcp"),
          [],
          context.cwd,
          new Set([
            "final_review.assess_risk",
            "final_review.plan",
            "final_review.filter_findings",
            "final_review.advance",
            "final_review.clean_status",
            "final_review.out_of_scope_report",
          ]),
        );
        const tiberTools = status.enabledFeatures.includes("tiber")
          ? await bridge(
              "tiber",
              path.join(packageRoot, "bin/tiber"),
              ["mcp", "stdio"],
              context.cwd,
              new Set([
                "tiber.create",
                "tiber.list",
                "tiber.search",
                "tiber.show",
                "tiber.next",
                "tiber.transition",
                "tiber.prioritize",
                "tiber.update",
                "tiber.validate_fix",
                "tiber.ci_recovery.claim",
                "tiber.ci_recovery.status",
                "tiber.ci_recovery.resolve",
              ]),
            )
          : [];
        context.ui.notify(
          `development_system.component_tools_active review=${reviewTools.length} tiber=${tiberTools.length}`,
          "info",
        );
      } catch (error) {
        context.ui.notify(
          error instanceof Error ? error.message : String(error),
          "error",
        );
      }
    }
  });

  pi.on("session_shutdown", async (_event, context) => {
    if (!started) return;
    started = false;
    for (const client of componentClients.splice(0)) client.stop();
    context.ui.setStatus("development-system", undefined);
  });
}
