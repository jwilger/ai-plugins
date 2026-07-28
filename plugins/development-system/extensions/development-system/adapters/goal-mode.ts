import type {
  ExtensionAPI,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  accountResponse,
  blockedDecision,
  completionDecision,
  goalPrompt,
  isGoalState,
  markContinuation,
  parseGoalCommand,
  pauseGoal,
  providerTokenTotal,
  resumeGoal,
  startGoal,
  type GoalState,
} from "../core/goal.ts";

const STATE_ENTRY = "development-system-goal-state";
const MESSAGE_TYPE = "development-system-goal-continuation";
const TERMINAL_TOOLS = ["goal_complete", "goal_blocked"] as const;
const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);

function now(): string {
  return new Date().toISOString();
}

function textOf(message: unknown): string {
  if (!message || typeof message !== "object") return "";
  const content = (message as { content?: unknown }).content;
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .filter(
      (item): item is { type: "text"; text: string } =>
        item?.type === "text" && typeof item.text === "string",
    )
    .map((item) => item.text)
    .join("\n");
}

function sessionTokenTotal(context: ExtensionContext): number {
  let total = 0;
  for (const entry of context.sessionManager.getBranch()) {
    if (entry.type !== "message") continue;
    const message = entry.message as unknown as {
      role?: string;
      usage?: unknown;
    };
    if (message.role === "assistant")
      total += providerTokenTotal(message.usage);
  }
  return total;
}

function formatStatus(state: GoalState | null): string {
  if (!state) return "development_system.goal status=none";
  return [
    "development_system.goal",
    `status=${state.status}`,
    `goal_id=${state.goalId}`,
    `responses=${state.automaticResponses}/${state.automaticLimit ?? "unlimited"}`,
    `tokens=${state.tokenUsage}/${state.tokenBudget ?? "unlimited"}`,
    `continuation=${state.continuationPending}`,
    state.stoppedReason ? `reason=${state.stoppedReason}` : "",
  ]
    .filter(Boolean)
    .join(" ");
}

/** Register session-scoped autonomous goal orchestration without filesystem or process effects. */
export function registerGoalMode(pi: ExtensionAPI): {
  current: () => GoalState | null;
  collision: boolean;
} {
  let state: GoalState | null = null;
  let ownedRunGoalId: string | null = null;
  let settledFailure: string | null = null;

  let collision = false;
  let collisionDiagnostic = "";

  const persist = (next: GoalState | null) => {
    state = next;
    pi.appendEntry(STATE_ENTRY, next ?? { cleared: true, schemaVersion: 1 });
  };

  const restore = (context: ExtensionContext) => {
    state = null;
    for (const entry of context.sessionManager?.getBranch?.() ?? []) {
      if (entry.type !== "custom" || entry.customType !== STATE_ENTRY) continue;
      state = isGoalState(entry.data) ? entry.data : null;
    }
    ownedRunGoalId = null;
    settledFailure = null;
  };

  const dispatch = (current: GoalState, continuation: boolean) => {
    ownedRunGoalId = current.goalId;
    pi.sendMessage(
      {
        customType: MESSAGE_TYPE,
        content: goalPrompt(current, continuation),
        display: true,
        details: {
          goalId: current.goalId,
          guardEpoch: current.guardEpoch,
          continuation,
        },
      },
      { triggerTurn: true, deliverAs: "followUp" },
    );
  };

  pi.on("session_start", async (_event, context) => {
    restore(context);
    const goalCommands = pi
      .getCommands()
      .filter((command) => command.name === "goal");
    const ownGoalCommands = goalCommands.filter(
      (command) => command.sourceInfo?.baseDir === packageRoot,
    );
    const reservedTools = pi
      .getAllTools()
      .filter((tool) => TERMINAL_TOOLS.includes(tool.name as never));
    const ownReservedTools = reservedTools.filter(
      (tool) => tool.sourceInfo?.baseDir === packageRoot,
    );
    collision =
      goalCommands.length !== 1 ||
      ownGoalCommands.length !== 1 ||
      reservedTools.length !== 2 ||
      ownReservedTools.length !== 2;
    collisionDiagnostic = `commands=${goalCommands.length}/${ownGoalCommands.length} tools=${reservedTools.length}/${ownReservedTools.length}`;
    if (collision) {
      if (state?.status === "active")
        persist(pauseGoal(state, now(), "reserved-name-collision"));
      context.ui.notify(
        `development_system.goal_collision ${collisionDiagnostic}; autonomous goal activation disabled`,
        "error",
      );
      return;
    }
    const active = new Set(pi.getActiveTools());
    const ordinaryDefault = ["read", "bash", "edit", "write"].every((tool) =>
      active.has(tool),
    );
    if (ordinaryDefault)
      pi.setActiveTools([...new Set([...active, ...TERMINAL_TOOLS])]);
  });
  pi.on("session_tree", async (_event, context) => restore(context));

  pi.registerCommand("goal", {
    description:
      "Start, inspect, pause, resume, or clear one bounded autonomous goal",
    handler: async (arguments_, context) => {
      try {
        if (collision)
          throw new Error(
            `development_system.goal_collision ${collisionDiagnostic}`,
          );
        const command = parseGoalCommand(arguments_);
        if (command.operation === "status") {
          context.ui.notify(formatStatus(state), "info");
          return;
        }
        if (command.operation === "clear") {
          persist(null);
          context.ui.notify("development_system.goal status=cleared", "info");
          return;
        }
        if (command.operation === "pause") {
          if (!state || state.status !== "active")
            throw new Error("development_system.goal_active_required");
          persist(pauseGoal(state, now(), "user-pause"));
          context.abort();
          context.ui.notify(formatStatus(state), "warning");
          return;
        }
        if (command.operation === "resume") {
          if (!state || state.status === "active")
            throw new Error("development_system.goal_stopped_required");
          const resumed = resumeGoal(state, {
            goalId: randomUUID(),
            guardEpoch: randomUUID(),
            now: now(),
            tokenBudget: command.tokenBudget,
            automaticLimit: command.automaticLimit,
          });
          persist(resumed);
          dispatch(resumed, false);
          context.ui.notify(formatStatus(resumed), "info");
          return;
        }
        const started = startGoal({
          objective: command.objective,
          goalId: randomUUID(),
          guardEpoch: randomUUID(),
          now: now(),
          tokenBaseline: sessionTokenTotal(context),
          tokenBudget: command.tokenBudget,
          automaticLimit: command.automaticLimit,
        });
        persist(started);
        dispatch(started, false);
        context.ui.notify(formatStatus(started), "info");
      } catch (error) {
        context.ui.notify(
          error instanceof Error ? error.message : String(error),
          "error",
        );
      }
    },
  });

  pi.registerTool({
    name: "goal_complete",
    label: "Complete Autonomous Goal",
    description:
      "Complete the exact active autonomous goal only after auditing every requirement and providing direct verification evidence.",
    parameters: {
      type: "object",
      properties: {
        goal_id: { type: "string", minLength: 1 },
        summary: { type: "string", minLength: 1, maxLength: 4_000 },
      },
      required: ["goal_id", "summary"],
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, _signal, _onUpdate, context) {
      if (collision)
        throw new Error(
          `development_system.goal_collision ${collisionDiagnostic}`,
        );
      if (
        typeof parameters.goal_id !== "string" ||
        typeof parameters.summary !== "string"
      )
        throw new Error("development_system.goal_completion_arguments_invalid");
      const completed = completionDecision(state, {
        goalId: parameters.goal_id,
        summary: parameters.summary,
        now: now(),
      });
      persist(completed);
      context.abort();
      return {
        content: [{ type: "text", text: formatStatus(completed) }],
        details: completed,
      };
    },
  });

  pi.registerTool({
    name: "goal_blocked",
    label: "Stop Autonomous Goal for External Blocker",
    description:
      "Stop the exact active goal only after the same external blocker persisted for at least three attempts and requires user or external action.",
    parameters: {
      type: "object",
      properties: {
        goal_id: { type: "string", minLength: 1 },
        reason: { type: "string", minLength: 1, maxLength: 1_000 },
        evidence: { type: "string", minLength: 1, maxLength: 4_000 },
        repeated_turns: { type: "integer", minimum: 3 },
      },
      required: ["goal_id", "reason", "evidence", "repeated_turns"],
      additionalProperties: false,
    },
    async execute(_toolCallId, parameters, _signal, _onUpdate, context) {
      if (collision)
        throw new Error(
          `development_system.goal_collision ${collisionDiagnostic}`,
        );
      if (
        typeof parameters.goal_id !== "string" ||
        typeof parameters.reason !== "string" ||
        typeof parameters.evidence !== "string" ||
        typeof parameters.repeated_turns !== "number"
      )
        throw new Error("development_system.goal_blocked_arguments_invalid");
      const blocked = blockedDecision(state, {
        goalId: parameters.goal_id,
        reason: parameters.reason,
        evidence: parameters.evidence,
        repeatedTurns: parameters.repeated_turns,
        now: now(),
      });
      persist(blocked);
      context.abort();
      return {
        content: [{ type: "text", text: formatStatus(blocked) }],
        details: blocked,
      };
    },
  });

  pi.on("input", async (event) => {
    if (!state || state.status !== "active" || event.source === "extension")
      return;
    // A direct user intervention starts a fresh stale-turn and repetition epoch.
    persist(
      resumeGoal(state, {
        goalId: randomUUID(),
        guardEpoch: randomUUID(),
        now: now(),
      }),
    );
  });

  pi.on("before_agent_start", async (event, context) => {
    const goalAuthored = event.prompt.startsWith(
      "DEVELOPMENT SYSTEM AUTONOMOUS GOAL",
    );
    if (!state || state.status !== "active") {
      if (goalAuthored) context.abort();
      return;
    }
    const ownershipPayload = JSON.stringify({
      goal_id: state.goalId,
      guard_epoch: state.guardEpoch,
      objective: state.objective,
    });
    if (goalAuthored && !event.prompt.includes(ownershipPayload)) {
      context.abort();
      return;
    }
    const activeTools = new Set(pi.getActiveTools());
    if (TERMINAL_TOOLS.some((tool) => !activeTools.has(tool))) {
      persist(pauseGoal(state, now(), "terminal-tools-unavailable"));
      context.abort();
      return;
    }
    ownedRunGoalId = state.goalId;
    settledFailure = null;
    return {
      systemPrompt: `${event.systemPrompt}\n\n${goalPrompt(state, false)}`,
    };
  });

  pi.on("turn_end", async (event) => {
    if (
      !state ||
      state.status !== "active" ||
      ownedRunGoalId !== state.goalId ||
      (event.message as { role?: string }).role !== "assistant"
    )
      return;
    const message = event.message as unknown as { usage?: unknown };
    persist(
      accountResponse(state, {
        output: textOf(event.message),
        hadToolActivity: event.toolResults.length > 0,
        providerTokens: providerTokenTotal(message.usage),
        now: now(),
      }),
    );
  });

  pi.on("agent_end", async (event) => {
    if (!state || state.status !== "active" || ownedRunGoalId !== state.goalId)
      return;
    const last = [...event.messages]
      .reverse()
      .find(
        (message) => (message as { role?: string }).role === "assistant",
      ) as { stopReason?: string; errorMessage?: string } | undefined;
    if (last?.stopReason === "error")
      settledFailure = /usage|rate.?limit|quota/i.test(last.errorMessage ?? "")
        ? "provider-usage-limit"
        : "terminal-provider-error";
    else if (last?.stopReason === "aborted")
      settledFailure = "user-interruption";
    if (state.status === "active")
      persist(markContinuation(state, true, now()));
  });

  pi.on("agent_settled", async (_event, context) => {
    if (!state || state.status !== "active" || !state.continuationPending)
      return;
    if (settledFailure) {
      persist(pauseGoal(state, now(), settledFailure));
      return;
    }
    if (!context.isIdle() || context.hasPendingMessages()) return;
    const continuing = markContinuation(state, false, now());
    persist(continuing);
    ownedRunGoalId = null;
    dispatch(continuing, true);
  });

  pi.on("session_shutdown", async () => {
    ownedRunGoalId = null;
    settledFailure = null;
  });

  return {
    current: () => state,
    get collision() {
      return collision;
    },
  };
}
