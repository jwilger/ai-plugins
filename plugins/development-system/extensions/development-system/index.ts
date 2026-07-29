import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { randomUUID } from "node:crypto";
import { chmod, readFile, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { resolveStatus } from "./adapters/status-interpreter.ts";
import { applySetupPreview, createSetupPreview } from "./adapters/setup.ts";
import {
  resolveReviewRoute,
  reviewFailureResult,
  runReviewChild,
} from "./adapters/review-child.ts";
import { registerGoalMode } from "./adapters/goal-mode.ts";
import { activeCiRecoveryHold } from "./adapters/ci-hold.ts";
import {
  createWorktree,
  listWorktrees,
  removeWorktree,
  validateWorktreeForCleanup,
  type WorktreeRecord,
} from "./adapters/worktrees.ts";
import { switchWorktreeSession } from "./adapters/worktree-session.ts";
import { PI_REFERENCES, readPiReference } from "./adapters/references.ts";
import {
  McpClient,
  publicToolName,
  schemaIsAdmissible,
  type McpTool,
} from "./adapters/mcp-client.ts";
import type { HarnessMode } from "./core/status.ts";
import { parseProjectPolicy } from "./core/configuration.ts";
import {
  configuredWorktreeRoot,
  parseWorktreeBranch,
  parseWorktreeName,
} from "./core/worktrees.ts";
import {
  classifyPath,
  classifyShellCommand,
  classifyShellDelivery,
  deliveryDecision,
  guardMessage,
  worktreeTargetAllowed,
} from "./core/guards.ts";

const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

function terminalSafe(value: string): string {
  return value
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "")
    .replace(
      /\u001b(?:\[[0-?]*[ -/]*[@-~]|\][^\u0007]*(?:\u0007|\u001b\\))/g,
      "",
    )
    .slice(0, 1_000);
}

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
      kind === "outside"
        ? "Use development_system_pi_reference for installed Pi docs or an allowed path in the active checkout."
        : "Use development_system_policy_read for the authoritative project policy; other protected metadata and secrets remain blocked.",
  });
}

async function guardContext(cwd: string, mode: HarnessMode) {
  const status = await resolveStatus(cwd, packageRoot, mode);
  let policy = null;
  try {
    policy = parseProjectPolicy(
      await readFile(
        path.join(status.checkout.primary, ".development-system.toml"),
        "utf8",
      ),
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
  const delivery = classifyShellDelivery(command);
  const context = await guardContext(cwd, mode);
  if (delivery) {
    const rejection = deliveryDecision({
      mode: context.policy?.delivery.mode ?? null,
      branch: context.branch,
      trunk: context.policy?.delivery.trunkBranch ?? "",
      destructive: delivery.kind === "destructive-delivery",
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
    classification.kind === "read-only-discovery"
  ) {
    try {
      const target = await realpath(
        path.resolve(cwd, classification.targetPath),
      );
      const inventory = await listWorktrees(cwd);
      if (inventory.worktrees.some((worktree) => worktree.path === target))
        return null;
    } catch {
      // Missing and non-worktree targets do not gain read authority.
    }
    return guardMessage({
      code: "development_system.coordination_discovery_target_blocked",
      boundary: "coordination-checkout discovery",
      missing: "a registered primary or linked worktree target",
      nextAction:
        "Call development_system_worktree_list and inspect only a returned canonical path.",
    });
  }
  if (
    context.policy?.features.worktrees &&
    context.status.checkout.kind === "primary" &&
    classification.kind === "worktree-creation"
  ) {
    try {
      const root = configuredWorktreeRoot(
        context.status.checkout.primary,
        context.policy.worktrees.root,
      );
      const lexicalTarget = path.resolve(cwd, classification.targetPath);
      const relative = path.relative(root, lexicalTarget);
      parseWorktreeName(relative);
      parseWorktreeBranch(classification.branch);
      if (
        worktreeTargetAllowed({
          rawPath: classification.targetPath,
          cwd,
          primary: context.status.checkout.primary,
          configuredRoot: context.policy.worktrees.root,
        })
      )
        return null;
    } catch {
      // Invalid semantic values do not gain shell mutation authority.
    }
    return guardMessage({
      code: "development_system.coordination_worktree_target_blocked",
      boundary: "coordination-checkout worktree creation",
      missing:
        "a target contained by the primary checkout's .worktrees directory",
      nextAction:
        "Create the linked worktree under .worktrees/ with a new -b branch.",
    });
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
      nextAction:
        "Call development_system_worktree_list, create one with development_system_worktree_create if needed, then run its switchCommand in the local Pi TUI to preserve this conversation without relaunching. Headless modes use relaunchCommand.",
    });
  }
  return null;
}

async function recordProvenanceMarker(goalCollision: boolean): Promise<void> {
  const marker = process.env.DEVELOPMENT_SYSTEM_PI_EVAL_MARKER;
  if (!marker) return;
  if (!path.isAbsolute(marker))
    throw new Error("development_system.eval_marker_requires_absolute_path");
  const packageMetadata = JSON.parse(
    await readFile(path.join(packageRoot, "package.json"), "utf8"),
  ) as { version: string };
  await writeFile(
    marker,
    `${JSON.stringify({
      package: "development-system",
      extension: fileURLToPath(import.meta.url),
      version: packageMetadata.version,
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
  const pendingAutomaticSwitches = new Map<string, WorktreeRecord>();
  const pendingAutomaticFinishes = new Map<string, WorktreeRecord>();
  const goalMode = registerGoalMode(pi);

  const queueAutomaticSwitch = (worktree: WorktreeRecord): string => {
    const token = randomUUID();
    pendingAutomaticSwitches.set(token, worktree);
    pi.sendUserMessage(
      `/development-system-worktree-switch --automatic ${token}`,
      { deliverAs: "followUp" },
    );
    return token;
  };

  const queueAutomaticFinish = (worktree: WorktreeRecord): string => {
    const token = randomUUID();
    pendingAutomaticFinishes.set(token, worktree);
    pi.sendUserMessage(
      `/development-system-worktree-finish --automatic ${token}`,
      { deliverAs: "followUp" },
    );
    return token;
  };

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

  pi.registerCommand("development-system-worktree-switch", {
    description:
      "Switch this Pi conversation to another registered worktree without relaunching",
    handler: async (arguments_, context) => {
      if (context.mode !== "tui" || !context.hasUI) {
        context.ui.notify(
          "development_system.worktree_switch_requires_local_tui: workspace replacement requires the local Pi TUI runtime.",
          "error",
        );
        return;
      }
      const selector = arguments_.trim();
      if (selector.length > 500 || /[\u0000-\u001f\u007f]/.test(selector)) {
        context.ui.notify(
          "development_system.worktree_switch_selector_invalid",
          "error",
        );
        return;
      }
      try {
        await context.waitForIdle();
        const inventory = await listWorktrees(context.cwd);
        const candidates = inventory.worktrees.filter(
          (worktree) => !worktree.current,
        );
        const automaticMatch = selector.match(
          /^--automatic ([0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12})$/i,
        );
        const automatic = automaticMatch !== null;
        let selected: WorktreeRecord | undefined;
        if (automaticMatch) {
          selected = pendingAutomaticSwitches.get(automaticMatch[1]);
          pendingAutomaticSwitches.delete(automaticMatch[1]);
          if (!selected)
            throw new Error(
              "development_system.worktree_switch_automatic_request_stale",
            );
        } else if (selector) {
          const matches = candidates.filter(
            (worktree) =>
              worktree.path === selector ||
              worktree.branch === selector ||
              path.basename(worktree.path) === selector,
          );
          if (matches.length === 0)
            throw new Error(
              `development_system.worktree_switch_not_found selector=${selector}`,
            );
          if (matches.length > 1)
            throw new Error(
              `development_system.worktree_switch_ambiguous selector=${selector}; use the exact branch or canonical path`,
            );
          selected = matches[0];
        } else {
          if (candidates.length === 0)
            throw new Error(
              "development_system.worktree_switch_none_available",
            );
          const labels = candidates.map(
            (worktree, index) =>
              `${index + 1}. ${terminalSafe(worktree.branch ?? "detached")} — ${terminalSafe(worktree.path)}`,
          );
          const choice = await context.ui.select(
            "Switch this Pi conversation to a registered worktree",
            labels,
          );
          if (!choice) return;
          selected = candidates[labels.indexOf(choice)];
          if (!selected)
            throw new Error(
              "development_system.worktree_switch_selection_stale",
            );
        }
        if (!selected)
          throw new Error("development_system.worktree_switch_selection_stale");
        if (!automatic) {
          const confirmed = await context.ui.confirm(
            "Switch Pi workspace?",
            `Preserve this conversation and active session branch in ${terminalSafe(selected.path)} (${terminalSafe(selected.branch ?? "detached")})? Pi will rebuild its cwd-bound runtime and re-evaluate project trust and resources there.`,
          );
          if (!confirmed) return;
        }

        const current = await listWorktrees(context.cwd);
        const revalidated = current.worktrees.find(
          (worktree) =>
            worktree.path === selected.path &&
            worktree.branch === selected.branch &&
            worktree.head === selected.head,
        );
        if (!revalidated || revalidated.current)
          throw new Error(
            "development_system.worktree_switch_identity_changed; list worktrees and select again",
          );
        await switchWorktreeSession(context, revalidated.path);
      } catch (error) {
        context.ui.notify(
          terminalSafe(error instanceof Error ? error.message : String(error)),
          "error",
        );
      }
    },
  });

  pi.registerCommand("development-system-worktree-finish", {
    description:
      "Switch this Pi conversation to the primary checkout and remove the finished clean linked worktree",
    handler: async (arguments_, context) => {
      if (context.mode !== "tui" || !context.hasUI) {
        context.ui.notify(
          "development_system.worktree_finish_requires_local_tui",
          "error",
        );
        return;
      }
      const selector = arguments_.trim();
      if (selector.length > 500 || /[\u0000-\u001f\u007f]/.test(selector)) {
        context.ui.notify(
          "development_system.worktree_finish_selector_invalid",
          "error",
        );
        return;
      }
      try {
        await context.waitForIdle();
        const inventory = await listWorktrees(context.cwd);
        const automaticMatch = selector.match(
          /^--automatic ([0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12})$/i,
        );
        const automatic = automaticMatch !== null;
        let selected: WorktreeRecord | undefined;
        if (automaticMatch) {
          selected = pendingAutomaticFinishes.get(automaticMatch[1]);
          pendingAutomaticFinishes.delete(automaticMatch[1]);
          if (!selected)
            throw new Error(
              "development_system.worktree_finish_automatic_request_stale",
            );
        } else if (!selector) {
          selected = inventory.worktrees.find((worktree) => worktree.current);
        } else {
          throw new Error(
            "development_system.worktree_finish_selector_invalid",
          );
        }
        if (!selected || selected.primary || !selected.current)
          throw new Error(
            "development_system.worktree_finish_requires_linked_checkout",
          );
        const validated = await validateWorktreeForCleanup(
          context.cwd,
          selected.path,
        );
        if (
          validated.branch !== selected.branch ||
          validated.head !== selected.head ||
          !validated.current
        )
          throw new Error(
            "development_system.worktree_cleanup_identity_changed",
          );
        if (!automatic) {
          const confirmed = await context.ui.confirm(
            "Finish and remove this worktree?",
            `Switch this conversation to ${terminalSafe(inventory.primary)}, run repository teardown, and remove the clean worktree ${terminalSafe(selected.path)}? The branch ${terminalSafe(selected.branch ?? "detached")} will be preserved.`,
          );
          if (!confirmed) return;
        }
        await switchWorktreeSession(
          context,
          inventory.primary,
          async (replacement) => {
            try {
              const removed = await removeWorktree(replacement.cwd, validated);
              replacement.ui.notify(
                `Removed finished worktree ${terminalSafe(removed.path)}. Branch ${terminalSafe(removed.branch ?? "detached")} was preserved.`,
                "info",
              );
            } catch (error) {
              replacement.ui.notify(
                `Workspace returned to the primary checkout, but cleanup preserved the worktree: ${terminalSafe(error instanceof Error ? error.message : String(error))}`,
                "error",
              );
            }
          },
        );
      } catch (error) {
        context.ui.notify(
          terminalSafe(error instanceof Error ? error.message : String(error)),
          "error",
        );
      }
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
    name: "development_system_policy_read",
    label: "Read Development System Policy",
    description:
      "Read only the authoritative .development-system.toml from the canonical primary checkout and return its parsed non-secret workflow policy.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      const policyPath = path.join(
        status.checkout.primary,
        ".development-system.toml",
      );
      const canonicalPolicy = await realpath(policyPath);
      if (path.dirname(canonicalPolicy) !== status.checkout.primary)
        throw new Error("development_system.policy_symlink_escape");
      const source = await readFile(canonicalPolicy, "utf8");
      const policy = parseProjectPolicy(source);
      return {
        content: [{ type: "text", text: source }],
        details: { path: canonicalPolicy, policy },
      };
    },
  });

  pi.registerTool({
    name: "development_system_pi_reference",
    label: "Read Installed Pi Reference",
    description:
      "Read a bounded page from an allowlisted installed Pi documentation file. Use nextOffset until the complete required reference has been read.",
    parameters: {
      type: "object",
      properties: {
        document: { type: "string", enum: Object.keys(PI_REFERENCES) },
        offset: { type: "integer", minimum: 1 },
        limit: { type: "integer", minimum: 1, maximum: 2_000 },
      },
      required: ["document"],
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, signal) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const result = await readPiReference({
        document: parameters.document,
        offset: parameters.offset,
        limit: parameters.limit,
      });
      return {
        content: [{ type: "text", text: result.lines.join("\n") }],
        details: {
          document: result.document,
          path: result.path,
          offset: result.offset,
          totalLines: result.totalLines,
          nextOffset: result.nextOffset,
        },
      };
    },
  });

  pi.registerTool({
    name: "development_system_worktree_list",
    label: "List Development Worktrees",
    description:
      "List canonical linked worktrees, branches, the authoritative current checkout, local-TUI conversation switch commands, and headless relaunch fallbacks. This is the supported primary-checkout discovery path.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const inventory = await listWorktrees(context.cwd);
      const automaticSwitchAvailable = context.mode === "tui" && context.hasUI;
      const result = {
        ...inventory,
        requiresUserWorkspaceSwitch:
          inventory.requiresUserWorkspaceSwitch && !automaticSwitchAvailable,
        processCwdImmutable: true,
        sessionWorkspaceSwitchAvailable: automaticSwitchAvailable,
        nextAction: automaticSwitchAvailable
          ? inventory.currentKind === "primary"
            ? "Use development_system_worktree_create if needed, or development_system_worktree_switch for an existing worktree. Development-system will preserve this conversation and replace the cwd-bound runtime automatically."
            : "Continue ordinary work in the current linked worktree, use development_system_worktree_switch to move, or call development_system_worktree_finish after verified delivery to return and clean up."
          : "Start a new Pi process with a returned relaunchCommand; this mode cannot replace the active cwd-bound runtime.",
      } as const;
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        details: result,
      };
    },
  });

  pi.registerTool({
    name: "development_system_worktree_switch",
    label: "Switch Development Worktree",
    description:
      "Queue an automatic local-TUI replacement of this Pi conversation into one exact registered worktree. No manual slash command is required.",
    parameters: {
      type: "object",
      properties: {
        selector: { type: "string", minLength: 1, maxLength: 500 },
      },
      required: ["selector"],
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      if (context.mode !== "tui" || !context.hasUI)
        throw new Error(
          "development_system.worktree_switch_requires_local_tui",
        );
      if (
        typeof parameters.selector !== "string" ||
        /[\u0000-\u001f\u007f]/.test(parameters.selector)
      )
        throw new Error("development_system.worktree_switch_selector_invalid");
      const inventory = await listWorktrees(context.cwd);
      const matches = inventory.worktrees.filter(
        (worktree) =>
          !worktree.current &&
          (worktree.path === parameters.selector ||
            worktree.branch === parameters.selector ||
            path.basename(worktree.path) === parameters.selector),
      );
      if (matches.length === 0)
        throw new Error(
          `development_system.worktree_switch_not_found selector=${parameters.selector}`,
        );
      if (matches.length > 1)
        throw new Error(
          `development_system.worktree_switch_ambiguous selector=${parameters.selector}`,
        );
      queueAutomaticSwitch(matches[0]);
      const result = {
        status: "queued",
        target: matches[0].path,
        branch: matches[0].branch,
        requiresUserWorkspaceSwitch: false,
        nextAction:
          "Finish the current response; development-system will replace the Pi workspace automatically.",
      } as const;
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        details: result,
      };
    },
  });

  pi.registerTool({
    name: "development_system_worktree_finish",
    label: "Finish Development Worktree",
    description:
      "After verified delivery is complete, queue automatic return to the primary checkout, run repository teardown, and remove the current clean linked worktree while preserving its branch.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      if (context.mode !== "tui" || !context.hasUI)
        throw new Error(
          "development_system.worktree_finish_requires_local_tui",
        );
      const inventory = await listWorktrees(context.cwd);
      const current = inventory.worktrees.find((worktree) => worktree.current);
      if (!current || current.primary)
        throw new Error(
          "development_system.worktree_finish_requires_linked_checkout",
        );
      const validated = await validateWorktreeForCleanup(
        context.cwd,
        current.path,
      );
      queueAutomaticFinish(validated);
      const result = {
        status: "queued",
        target: validated.path,
        branch: validated.branch,
        branchPreserved: true,
        nextAction:
          "Finish the current response; development-system will return to the primary checkout and remove the clean worktree automatically.",
      } as const;
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        details: result,
      };
    },
  });

  pi.registerTool({
    name: "development_system_worktree_create",
    label: "Create Development Worktree",
    description:
      "Create one new repository-local linked worktree from the authoritative primary HEAD. In local TUI mode, queue automatic conversation replacement into it; headless callers receive a relaunch fallback.",
    parameters: {
      type: "object",
      properties: {
        name: {
          type: "string",
          minLength: 1,
          maxLength: 100,
          pattern: "^[A-Za-z0-9][A-Za-z0-9._-]*$",
        },
        branch: { type: "string", minLength: 1, maxLength: 255 },
      },
      required: ["name", "branch"],
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const result = await createWorktree(context.cwd, {
        name: parameters.name,
        branch: parameters.branch,
      });
      if (
        result.status === "created" &&
        context.mode === "tui" &&
        context.hasUI
      ) {
        const created = (await listWorktrees(context.cwd)).worktrees.find(
          (worktree) => worktree.path === result.path,
        );
        if (!created)
          throw new Error("development_system.worktree_creation_unobservable");
        queueAutomaticSwitch(created);
        const queued = {
          ...result,
          requiresUserWorkspaceSwitch: false,
          switchQueued: true,
          nextAction:
            "Automatic local-TUI workspace replacement is queued for the end of this response.",
        } as const;
        return {
          content: [{ type: "text", text: JSON.stringify(queued) }],
          details: queued,
        };
      }
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        details: result,
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
    async execute(_toolCallId, parameters, signal, onUpdate, context) {
      if (
        typeof parameters.assignment !== "string" ||
        typeof parameters.model_role !== "string"
      ) {
        throw new Error("development_system.review_assignment_invalid");
      }
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      const policy = parseProjectPolicy(
        await readFile(
          path.join(status.checkout.primary, ".development-system.toml"),
          "utf8",
        ),
      );
      const route = resolveReviewRoute(
        parameters.model_role,
        policy.piReviewModels,
      );
      onUpdate?.({
        content: [
          {
            type: "text",
            text: JSON.stringify({
              status: "running",
              lifecycle: { state: "starting-fresh-child" },
              model_role: parameters.model_role,
              route: { provider: route.provider, model: route.model },
            }),
          },
        ],
        details: {
          status: "running",
          state: "starting-fresh-child",
          modelRole: parameters.model_role,
        },
      });
      try {
        const result = await runReviewChild({
          assignment: {
            assignment: parameters.assignment,
            modelRole: parameters.model_role,
          },
          cwd: context.cwd,
          route,
          signal,
        });
        return {
          content: [{ type: "text", text: JSON.stringify(result) }],
          details: result,
        };
      } catch (error) {
        const failure = reviewFailureResult(error);
        if (!failure) throw error;
        return {
          content: [{ type: "text", text: JSON.stringify(failure) }],
          details: failure,
        };
      }
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
        const authoritativePolicy = path.join(
          status.checkout.primary,
          ".development-system.toml",
        );
        const allowedPolicyRead =
          event.toolName === "read" &&
          classified.kind === "protected-metadata" &&
          classified.canonicalPath === authoritativePolicy;
        if (classified.kind !== "inside" && !allowedPolicyRead)
          return { block: true, reason: pathRejection(classified.kind) };
        if (
          ["write", "edit"].includes(event.toolName) &&
          status.checkout.kind === "primary"
        ) {
          let worktrees = false;
          try {
            worktrees = parseProjectPolicy(
              await readFile(authoritativePolicy, "utf8"),
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
                nextAction:
                  "Call development_system_worktree_list, create one with development_system_worktree_create if needed, then relaunch Pi with the returned command.",
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
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      const policy = parseProjectPolicy(
        await readFile(
          path.join(status.checkout.primary, ".development-system.toml"),
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
      "development_system_policy_read",
      "development_system_pi_reference",
      "development_system_worktree_list",
      "development_system_worktree_switch",
      "development_system_worktree_finish",
      "development_system_worktree_create",
      "development_system_run_review_assignment",
      "development_system_goal_status",
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
