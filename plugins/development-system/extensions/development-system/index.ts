import {
  createLocalBashOperations,
  type ExtensionAPI,
} from "@earendil-works/pi-coding-agent";
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
import { LogicalWorkspaceAuthority } from "./adapters/logical-workspace.ts";
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
  deliveryExplicitlyTargetsTrunk,
  guardMessage,
  worktreeTargetAllowed,
} from "./core/guards.ts";

const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'"'"'`)}'`;
}

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
      explicitTrunkTarget: deliveryExplicitlyTargetsTrunk(
        command,
        context.policy?.delivery.trunkBranch ?? "",
      ),
    });
    if (
      rejection?.code === "development_system.destructive_approval_required" &&
      mode === "tui"
    ) {
      const approved = await confirm(
        "Approve destructive delivery operation?",
        command,
      );
      if (!approved) return guardMessage(rejection);
    } else if (rejection) {
      return guardMessage(rejection);
    }
    // A permitted delivery embedded in a compound command does not authorize
    // earlier filesystem or Git mutation from the coordination checkout.
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
        "Call development_system_worktree_list, then create or activate a registered linked logical workspace. Pi remains in this conversation and requires no relaunch.",
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
  const goalMode = registerGoalMode(pi);
  const workspace = new LogicalWorkspaceAuthority((customType, data) =>
    pi.appendEntry(customType, data),
  );
  const projectCwd = (hostCwd: string): string => workspace.path(hostCwd);

  const assertLifecycleToolIsolated = (
    context: {
      sessionManager?: {
        getBranch?(): readonly {
          type?: string;
          message?: { role?: string; content?: unknown };
        }[];
      };
    },
    toolName: string,
  ): void => {
    const branch = context.sessionManager?.getBranch?.();
    if (!branch) return;
    const assistant = [...branch]
      .reverse()
      .find(
        (entry) =>
          entry.type === "message" && entry.message?.role === "assistant",
      )?.message;
    if (!assistant) return;
    const calls = Array.isArray(assistant.content)
      ? assistant.content.filter(
          (item): item is { type: "toolCall"; name: string } =>
            !!item &&
            typeof item === "object" &&
            (item as { type?: unknown }).type === "toolCall" &&
            typeof (item as { name?: unknown }).name === "string",
        )
      : [];
    if (calls.length !== 1 || calls[0].name !== toolName)
      throw new Error(
        `development_system.worktree_lifecycle_tool_must_be_isolated tool=${toolName}; retry it as the only tool call in one assistant message`,
      );
  };

  const activateWorkspace = async (
    context: {
      cwd: string;
      ui?: { setStatus?(key: string, value: string): void };
    },
    target: string,
    expectedBranch?: string | null,
  ) => {
    const activated = await workspace.activate(context, target, expectedBranch);
    context.ui?.setStatus?.(
      "development-system-workspace",
      `workspace: ${path.basename(activated.path)}`,
    );
    return activated;
  };

  const assertHostDoesNotDependOn = async (
    hostCwd: string,
    target: string,
  ): Promise<void> => {
    const hostInventory = await listWorktrees(hostCwd);
    const host = hostInventory.worktrees.find((worktree) => worktree.current);
    if (host?.path === target)
      throw new Error(
        `development_system.worktree_finish_host_checkout_migration_required primary=${hostInventory.primary}; this Pi process was launched in the worktree that cleanup would remove, so start Pi from the primary checkout once before retrying finish`,
      );
  };

  const bridge = async (
    origin: "tiber" | "review",
    command: string,
    args: readonly string[],
    cwd: string,
    approvedNames: ReadonlySet<string>,
  ): Promise<readonly string[]> => {
    const client = new McpClient({ command, args, cwd });
    let discovered: readonly McpTool[];
    try {
      await client.start();
      discovered = await client.listTools();
    } finally {
      client.stop();
    }
    const tools = discovered.filter((tool) => approvedNames.has(tool.name));
    const admitted = tools.filter((tool) =>
      schemaIsAdmissible(tool.inputSchema),
    );
    const existing = new Set(pi.getAllTools().map((tool) => tool.name));
    const registrations = admitted.map((tool) => ({
      tool,
      publicName: publicToolName(origin, tool.name),
    }));
    if (registrations.some(({ publicName }) => existing.has(publicName)))
      throw new Error(`development_system.mcp_tool_collision origin=${origin}`);
    for (const { tool, publicName } of registrations) {
      pi.registerTool({
        name: publicName,
        label: `${origin === "tiber" ? "Tiber" : "Final Review"}: ${tool.name}`,
        description:
          tool.description ?? `First-party ${origin} operation ${tool.name}`,
        parameters: tool.inputSchema as McpTool["inputSchema"],
        async execute(_toolCallId, parameters, signal, _onUpdate, context) {
          const routed = new McpClient({
            command,
            args,
            cwd: projectCwd(context.cwd),
          });
          await routed.start();
          try {
            const result = await routed.callTool(tool.name, parameters, signal);
            const text = JSON.stringify(result);
            if (Buffer.byteLength(text) > 50 * 1024)
              throw new Error("development_system.component_output_limit");
            return { content: [{ type: "text", text }], details: result };
          } finally {
            routed.stop();
          }
        },
      });
    }
    return registrations.map(({ publicName }) => publicName);
  };

  pi.registerCommand("development-system-status", {
    description: "Show deterministic development-system project status",
    handler: async (_arguments, context) => {
      const status = await resolveStatus(
        projectCwd(context.cwd),
        packageRoot,
        context.mode as HarnessMode,
      );
      context.ui.notify(
        JSON.stringify(
          {
            ...status,
            hostCwd: context.cwd,
            logicalWorkspace: projectCwd(context.cwd),
            goal: goalMode.current(),
          },
          null,
          2,
        ),
        status.errors.length > 0 ? "warning" : "info",
      );
    },
  });

  pi.registerCommand("development-system-worktree-create", {
    description:
      "Create a linked worktree and make it this session's logical workspace",
    handler: async (arguments_, context) => {
      const [name, branch, ...extra] = arguments_.trim().split(/\s+/);
      if (!name || !branch || extra.length > 0) {
        context.ui.notify(
          "development_system.worktree_create_arguments_invalid expected=<name> <branch>",
          "error",
        );
        return;
      }
      try {
        await context.waitForIdle();
        if (context.mode === "tui" && context.hasUI) {
          const confirmed = await context.ui.confirm(
            "Create and activate logical workspace?",
            `Create ${terminalSafe(name)} on new branch ${terminalSafe(branch)} and route this session into it?`,
          );
          if (!confirmed) return;
        }
        const created = await createWorktree(projectCwd(context.cwd), {
          name,
          branch,
        });
        if (created.status !== "created")
          throw new Error(`${created.code}: ${created.nextAction}`);
        const activated = await activateWorkspace(
          context,
          created.path,
          created.branch,
        );
        context.ui.notify(
          `Created and activated logical workspace ${terminalSafe(activated.path)} on branch ${terminalSafe(activated.branch ?? "detached")}. Pi's host cwd was unchanged.`,
          "info",
        );
      } catch (error) {
        context.ui.notify(
          terminalSafe(error instanceof Error ? error.message : String(error)),
          "error",
        );
      }
    },
  });

  pi.registerCommand("development-system-worktree-switch", {
    description:
      "Set this session's logical workspace to one registered linked worktree",
    handler: async (arguments_, context) => {
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
        try {
          await workspace.resolve(context.cwd);
        } catch {
          await workspace.resetToHost(context);
        }
        const inventory = await listWorktrees(projectCwd(context.cwd));
        const candidates = inventory.worktrees.filter(
          (worktree) => !worktree.current && !worktree.primary,
        );
        let selected: WorktreeRecord | undefined;
        if (selector) {
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
              `development_system.worktree_switch_ambiguous selector=${selector}`,
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
            "Use a registered linked worktree as this session's logical workspace",
            labels,
          );
          if (!choice) return;
          selected = candidates[labels.indexOf(choice)];
        }
        if (!selected)
          throw new Error("development_system.worktree_switch_selection_stale");
        if (context.mode === "tui" && context.hasUI) {
          const confirmed = await context.ui.confirm(
            "Switch logical workspace?",
            `Route subsequent shell and file tools into ${terminalSafe(selected.path)} without relaunching Pi?`,
          );
          if (!confirmed) return;
        }
        const activated = await activateWorkspace(
          context,
          selected.path,
          selected.branch,
        );
        context.ui.notify(
          `Logical workspace is now ${terminalSafe(activated.path)}. Pi's host working directory and conversation were unchanged.`,
          "info",
        );
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
      "Return logical routing to primary and remove the finished clean worktree",
    handler: async (arguments_, context) => {
      if (arguments_.trim()) {
        context.ui.notify(
          "development_system.worktree_finish_selector_invalid",
          "error",
        );
        return;
      }
      try {
        await context.waitForIdle();
        await workspace.finish(
          context,
          async (source) => {
            await assertHostDoesNotDependOn(context.cwd, source.path);
            const validated = await validateWorktreeForCleanup(
              source.path,
              source.path,
            );
            if (context.mode === "tui" && context.hasUI) {
              const confirmed = await context.ui.confirm(
                "Finish and remove this worktree?",
                `Return logical routing to ${terminalSafe(source.primary)}, run teardown, and remove ${terminalSafe(source.path)} while preserving branch ${terminalSafe(source.branch ?? "detached")}?`,
              );
              if (!confirmed) return null;
            }
            return validated;
          },
          async (source, primary, validated) => {
            context.ui.setStatus?.(
              "development-system-workspace",
              `workspace: ${path.basename(primary.path)}`,
            );
            try {
              const removed = await removeWorktree(
                primary.path,
                validated,
                () => workspace.assertIdentity(source),
              );
              context.ui.notify(
                `Removed finished worktree ${terminalSafe(removed.path)}. Branch ${terminalSafe(removed.branch ?? "detached")} was preserved.`,
                "info",
              );
            } catch (error) {
              context.ui.notify(
                `Logical routing returned to primary, but cleanup preserved the worktree: ${terminalSafe(error instanceof Error ? error.message : String(error))}`,
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
          projectCwd(context.cwd),
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
        projectCwd(context.cwd),
        packageRoot,
        context.mode as HarnessMode,
      );
      return {
        content: [{ type: "text", text: concise(status) }],
        details: {
          ...status,
          hostCwd: context.cwd,
          logicalWorkspace: projectCwd(context.cwd),
          goal: goalMode.current(),
        },
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
        projectCwd(context.cwd),
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
      "List canonical linked worktrees, branches, and this session's authoritative logical workspace.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const inventory = await listWorktrees(projectCwd(context.cwd));
      const result = {
        ...inventory,
        hostCwd: context.cwd,
        logicalWorkspace: inventory.current,
        processCwdImmutable: true,
        requiresUserWorkspaceSwitch: false,
        sessionWorkspaceSwitchAvailable: true,
        nextAction:
          inventory.currentKind === "primary"
            ? "Create a linked worktree or activate an existing one; no Pi relaunch is required."
            : "Continue in the logical linked worktree or finish it after verified delivery.",
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
      "Treat one exact registered linked worktree as this session's logical workspace without relaunching Pi.",
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
      assertLifecycleToolIsolated(
        context,
        "development_system_worktree_switch",
      );
      if (
        typeof parameters.selector !== "string" ||
        /[\u0000-\u001f\u007f]/.test(parameters.selector)
      )
        throw new Error("development_system.worktree_switch_selector_invalid");
      const inventory = await listWorktrees(projectCwd(context.cwd));
      const matches = inventory.worktrees.filter(
        (worktree) =>
          !worktree.current &&
          !worktree.primary &&
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
      const activated = await activateWorkspace(
        context,
        matches[0].path,
        matches[0].branch,
      );
      const result = {
        status: "active",
        target: activated.path,
        branch: activated.branch,
        hostCwd: context.cwd,
        logicalWorkspace: activated.path,
        requiresUserWorkspaceSwitch: false,
        nextAction:
          "Route every repository operation through the logical workspace; no relaunch or slash command is needed.",
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
      "After verified delivery, route this session back to primary, run teardown, and remove the clean linked worktree while preserving its branch.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      assertLifecycleToolIsolated(
        context,
        "development_system_worktree_finish",
      );
      const removed = await workspace.finish(
        context,
        async (source) => {
          await assertHostDoesNotDependOn(context.cwd, source.path);
          return validateWorktreeForCleanup(source.path, source.path);
        },
        async (source, primary, validated) => {
          context.ui?.setStatus?.(
            "development-system-workspace",
            `workspace: ${path.basename(primary.path)}`,
          );
          try {
            return await removeWorktree(primary.path, validated, () =>
              workspace.assertIdentity(source),
            );
          } catch (error) {
            throw new Error(
              `development_system.worktree_finish_cleanup_preserved logical_workspace=${primary.path} cause=${terminalSafe(error instanceof Error ? error.message : String(error))}`,
            );
          }
        },
      );
      if (!removed)
        throw new Error("development_system.worktree_finish_cancelled");
      const result = {
        status: "removed",
        target: removed.path,
        logicalWorkspace: workspace.path(context.cwd),
        branch: removed.branch,
        branchPreserved: true,
        hostCwd: context.cwd,
        nextAction: "Continue coordination from the primary logical workspace.",
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
      "Create a repository-local linked worktree from primary HEAD and immediately make it this session's logical workspace.",
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
      assertLifecycleToolIsolated(
        context,
        "development_system_worktree_create",
      );
      const result = await createWorktree(projectCwd(context.cwd), {
        name: parameters.name,
        branch: parameters.branch,
      });
      if (result.status !== "created")
        return {
          content: [{ type: "text", text: JSON.stringify(result) }],
          details: result,
        };
      const activated = await activateWorkspace(
        context,
        result.path,
        result.branch,
      );
      const active = {
        ...result,
        status: "created" as const,
        logicalWorkspace: activated.path,
        requiresUserWorkspaceSwitch: false,
        nextAction:
          "The linked worktree is active as this session's logical workspace; no relaunch is required.",
      } as const;
      return {
        content: [{ type: "text", text: JSON.stringify(active) }],
        details: active,
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
        projectCwd(context.cwd),
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
        projectCwd(context.cwd),
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
          cwd: projectCwd(context.cwd),
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
    let logicalCwd: string;
    try {
      logicalCwd = await workspace.resolve(context.cwd);
    } catch (error) {
      const recoveryTools = new Set([
        "development_system_status",
        "development_system_policy_read",
        "development_system_worktree_list",
        "development_system_worktree_switch",
        "development_system_worktree_create",
      ]);
      if (recoveryTools.has(event.toolName)) {
        try {
          logicalCwd = (await workspace.resetToHost(context)).path;
        } catch (recoveryError) {
          return {
            block: true,
            reason: guardMessage({
              code: "development_system.logical_workspace_recovery_failed",
              boundary: "logical workspace recovery",
              missing: "a registered host checkout",
              nextAction: terminalSafe(
                recoveryError instanceof Error
                  ? recoveryError.message
                  : String(recoveryError),
              ),
            }),
          };
        }
      } else
        return {
          block: true,
          reason: guardMessage({
            code: "development_system.logical_workspace_stale",
            boundary: "logical workspace routing",
            missing: "a currently registered logical workspace identity",
            nextAction: `Run development_system_worktree_list and activate a valid worktree before retrying. ${terminalSafe(error instanceof Error ? error.message : String(error))}`,
          }),
        };
    }
    if (["write", "edit", "bash"].includes(event.toolName)) {
      const current = await guardContext(
        logicalCwd,
        context.mode as HarnessMode,
      );
      if (current.policy?.features.tiber) {
        const hold = await activeCiRecoveryHold(packageRoot, logicalCwd);
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

    if (
      ["write", "edit", "read", "grep", "find", "ls"].includes(event.toolName)
    ) {
      const input = event.input as { path?: unknown };
      const rawPath =
        typeof input.path === "string"
          ? input.path
          : ["grep", "find", "ls"].includes(event.toolName)
            ? "."
            : null;
      if (rawPath !== null) {
        const status = await resolveStatus(
          logicalCwd,
          packageRoot,
          context.mode as HarnessMode,
        );
        const classified = classifyPath({
          rawPath,
          cwd: logicalCwd,
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
            // Missing policy grants nothing but does not impose worktree policy.
          }
          if (worktrees)
            return {
              block: true,
              reason: guardMessage({
                code: "development_system.coordination_write_blocked",
                boundary: "coordination checkout",
                missing: "an active linked logical workspace",
                nextAction:
                  "Create or activate a registered linked worktree with the development-system worktree tools; no Pi relaunch is required.",
              }),
            };
        }
        input.path = classified.canonicalPath;
      }
    }

    if (event.toolName === "bash") {
      const input = event.input as { command?: unknown };
      const command = input.command;
      if (typeof command === "string") {
        const reason = await shellRejection(
          command,
          logicalCwd,
          context.mode as HarnessMode,
          (title, message) => context.ui.confirm(title, message),
        );
        if (reason) return { block: true, reason };
        input.command = `cd -- ${shellQuote(logicalCwd)} && ${command}`;
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
    try {
      const logicalCwd = await workspace.resolve(context.cwd);
      const reason = await shellRejection(
        event.command,
        logicalCwd,
        context.mode as HarnessMode,
        (title, message) => context.ui.confirm(title, message),
      );
      if (reason)
        return {
          result: {
            output: reason,
            exitCode: 2,
            cancelled: false,
            truncated: false,
          },
        };
      const local = createLocalBashOperations();
      return {
        operations: {
          exec(command, _cwd, options) {
            return local.exec(command, logicalCwd, options);
          },
        },
      };
    } catch (error) {
      return {
        result: {
          output: guardMessage({
            code: "development_system.logical_workspace_user_bash_failed",
            boundary: "logical workspace routing",
            missing: "a reachable and policy-valid logical workspace",
            nextAction: `Run development_system_worktree_list and restore a valid logical workspace before retrying. ${terminalSafe(error instanceof Error ? error.message : String(error))}`,
          }),
          exitCode: 2,
          cancelled: false,
          truncated: false,
        },
      };
    }
  });

  pi.on("before_agent_start", async (event, context) => {
    const logicalCwd = projectCwd(context.cwd);
    const routedSystemPrompt = `${event.systemPrompt}\n\nDevelopment-system logical workspace: ${logicalCwd}. Pi's host cwd remains ${context.cwd}. Every built-in shell and relative file/search operation is independently routed to the logical workspace. Treat its files, branch, index, environment, and Git status as authoritative; do not request a relaunch merely because host cwd differs.`;
    try {
      const status = await resolveStatus(
        logicalCwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      const policy = parseProjectPolicy(
        await readFile(
          path.join(status.checkout.primary, ".development-system.toml"),
          "utf8",
        ),
      );
      if (!policy.features.tiber) return { systemPrompt: routedSystemPrompt };
      const hold = await activeCiRecoveryHold(packageRoot, logicalCwd);
      if (hold)
        return {
          systemPrompt: routedSystemPrompt,
          message: {
            customType: "development-system-ci-hold",
            content: `development_system.ci_recovery_hold incident=${hold.incidentId} state=${hold.state}. Do not claim readiness or start unrelated guarded work; continue authoritative Tiber recovery.`,
            display: true,
          },
        };
      return { systemPrompt: routedSystemPrompt };
    } catch {
      return { systemPrompt: routedSystemPrompt };
    }
  });

  pi.on("session_start", async (_event, context) => {
    started = true;
    await recordProvenanceMarker(goalMode.collision);
    const restoration = await workspace.restore(context);
    if (restoration.status === "rejected")
      context.ui.notify(
        restoration.code ?? "development_system.logical_workspace_stale",
        "warning",
      );
    const status = await resolveStatus(
      projectCwd(context.cwd),
      packageRoot,
      context.mode as HarnessMode,
    );
    context.ui.setStatus(
      "development-system",
      status.configured
        ? `development-system: ${status.deliveryMode}`
        : "development-system: setup required",
    );
    context.ui.setStatus(
      "development-system-workspace",
      `workspace: ${path.basename(projectCwd(context.cwd))}`,
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
          projectCwd(context.cwd),
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
              projectCwd(context.cwd),
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

  pi.on("session_tree", async (_event, context) => {
    const restoration = await workspace.restore(context);
    if (restoration.status === "rejected")
      context.ui.notify(
        restoration.code ?? "development_system.logical_workspace_stale",
        "warning",
      );
    context.ui.setStatus(
      "development-system-workspace",
      `workspace: ${path.basename(projectCwd(context.cwd))}`,
    );
  });

  pi.on("session_shutdown", async (_event, context) => {
    if (!started) return;
    started = false;
    context.ui.setStatus("development-system", undefined);
  });
}
