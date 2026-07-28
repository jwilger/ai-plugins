import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "../..");
const core = await import(
  path.join(
    root,
    "plugins/development-system/extensions/development-system/core/goal.ts",
  )
);
const { registerGoalMode } = await import(
  path.join(
    root,
    "plugins/development-system/extensions/development-system/adapters/goal-mode.ts",
  )
);

const pluginRoot = path.join(root, "plugins/development-system");
const id = "00000000-0000-4000-8000-000000000001";
const epoch = "00000000-0000-4000-8000-000000000002";
const time = "2026-07-28T00:00:00.000Z";

function active(overrides = {}) {
  return {
    ...core.startGoal({
      objective: "Implement and verify every requirement",
      goalId: id,
      guardEpoch: epoch,
      now: time,
      tokenBaseline: 10,
      tokenBudget: 1_000,
      automaticLimit: 25,
    }),
    ...overrides,
  };
}

test("goal command grammar is bounded and explicit", () => {
  assert.deepEqual(core.parseGoalCommand(""), { operation: "status" });
  assert.deepEqual(
    core.parseGoalCommand("--tokens 2k --turns unlimited ship it"),
    {
      operation: "start",
      objective: "ship it",
      tokenBudget: 2_000,
      automaticLimit: null,
    },
  );
  assert.deepEqual(core.parseGoalCommand("resume --tokens 3m --turns 4"), {
    operation: "resume",
    tokenBudget: 3_000_000,
    automaticLimit: 4,
  });
  assert.throws(() => core.parseGoalCommand("--turns 0 no"), /turns_invalid/);
  assert.throws(() => core.parseGoalCommand("x".repeat(4_001)), /too_long/);
});

test("goal response accounting pauses on each independent bound", () => {
  const turnLimited = core.accountResponse(active({ automaticLimit: 1 }), {
    output: "working",
    hadToolActivity: true,
    providerTokens: 5,
    now: time,
  });
  assert.equal(turnLimited.status, "paused");
  assert.equal(turnLimited.stoppedReason, "automatic-response-limit");

  const tokenLimited = core.accountResponse(active({ tokenBudget: 5 }), {
    output: "working",
    hadToolActivity: true,
    providerTokens: 5,
    now: time,
  });
  assert.equal(tokenLimited.stoppedReason, "token-budget-exhausted");

  let repeated = active();
  for (let index = 0; index < 3; index++) {
    repeated = core.accountResponse(repeated, {
      output: "same output",
      hadToolActivity: false,
      providerTokens: 1,
      now: time,
    });
  }
  assert.equal(repeated.stoppedReason, "repeated-empty-or-identical-output");
});

test("provider token accounting prefers total and never sums nested reasoning", () => {
  assert.equal(
    core.providerTokenTotal({
      totalTokens: 12,
      input: 7,
      output: 5,
      reasoning: { tokens: 99 },
    }),
    12,
  );
  assert.equal(
    core.providerTokenTotal({
      input: 3,
      output: 2,
      cacheRead: 4,
      cacheWrite: 1,
    }),
    10,
  );
  assert.equal(core.providerTokenTotal({ totalTokens: Number.NaN }), 0);
});

test("terminal decisions reject stale, premature, and weak blocker claims", () => {
  assert.throws(
    () =>
      core.completionDecision(active(), {
        goalId: "stale",
        summary: "all done",
        now: time,
      }),
    /stale/,
  );
  assert.throws(
    () =>
      core.completionDecision(active(), {
        goalId: id,
        summary: "Tests are still failing and work is incomplete",
        now: time,
      }),
    /contradictory/,
  );
  assert.throws(
    () =>
      core.blockedDecision(active(), {
        goalId: id,
        reason: "This is difficult and uncertain",
        evidence: "attempted",
        repeatedTurns: 3,
        now: time,
      }),
    /not_external/,
  );
  assert.throws(
    () =>
      core.blockedDecision(active(), {
        goalId: id,
        reason: "The implementation does not work",
        evidence: "Three attempts produced the same test result",
        repeatedTurns: 3,
        now: time,
      }),
    /not_external/,
  );
  assert.equal(
    core.blockedDecision(active(), {
      goalId: id,
      reason: "The external service requires owner account approval",
      evidence:
        "The same authorization response persisted on attempts 1, 2, and 3",
      repeatedTurns: 3,
      now: time,
    }).status,
    "blocked",
  );
});

function harness({ commands = [], tools = [] } = {}) {
  const handlers = new Map();
  const externalCommands = commands.map((name) => ({
    name,
    sourceInfo: { baseDir: "/other-package" },
  }));
  const registeredCommands = new Map();
  const registeredTools = tools.map((name) => ({
    name,
    sourceInfo: { baseDir: "/other-package" },
  }));
  const entries = [];
  const messages = [];
  const pi = {
    getCommands: () => [
      ...externalCommands,
      ...[...registeredCommands].map(([name, definition]) => ({
        name,
        sourceInfo: definition.sourceInfo,
      })),
    ],
    getAllTools: () => registeredTools,
    getActiveTools: () => registeredTools.map((tool) => tool.name),
    registerCommand(name, definition) {
      registeredCommands.set(name, {
        ...definition,
        sourceInfo: { baseDir: pluginRoot },
      });
    },
    registerTool(definition) {
      registeredTools.push({
        ...definition,
        sourceInfo: { baseDir: pluginRoot },
      });
    },
    on(name, handler) {
      const current = handlers.get(name) ?? [];
      handlers.set(name, [...current, handler]);
    },
    appendEntry(customType, data) {
      entries.push({ type: "custom", customType, data });
    },
    sendMessage(message, options) {
      messages.push({ message, options });
    },
  };
  const context = {
    mode: "tui",
    cwd: "/tmp",
    sessionManager: { getBranch: () => entries },
    ui: { notify() {} },
    abort() {},
    isIdle: () => true,
    hasPendingMessages: () => false,
  };
  const emit = async (name, event, customContext = context) => {
    let result;
    for (const handler of handlers.get(name) ?? []) {
      result = (await handler(event, customContext)) ?? result;
    }
    return result;
  };
  return {
    pi,
    handlers,
    registeredCommands,
    registeredTools,
    entries,
    messages,
    context,
    emit,
  };
}

test("goal adapter persists only in the session branch and dispatches one custom continuation", async () => {
  const runtime = harness();
  const mode = registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  await runtime.registeredCommands
    .get("goal")
    .handler("--turns 2 do the work", runtime.context);
  assert.equal(mode.current().status, "active");
  assert.equal(runtime.messages.length, 1);
  assert.equal(
    runtime.messages[0].message.customType,
    "development-system-goal-continuation",
  );

  await runtime.emit("before_agent_start", {
    systemPrompt: "base",
    prompt: runtime.messages[0].message.content,
  });
  await runtime.emit("turn_end", {
    message: {
      role: "assistant",
      content: [{ type: "text", text: "working" }],
      usage: { totalTokens: 2 },
    },
    toolResults: [{ role: "toolResult" }],
  });
  await runtime.emit("agent_end", {
    messages: [{ role: "assistant", stopReason: "stop" }],
  });
  await runtime.emit("agent_settled", {});
  await runtime.emit("agent_settled", {});
  assert.equal(runtime.messages.length, 2);
  assert.equal(runtime.messages[1].options.triggerTurn, true);
  assert.equal(mode.current().continuationPending, false);

  const restored = harness();
  restored.entries.push(...runtime.entries);
  const restoredMode = registerGoalMode(restored.pi);
  await restored.emit("session_start", {});
  assert.equal(restoredMode.current().objective, "do the work");
  const newSession = harness();
  const newMode = registerGoalMode(newSession.pi);
  await newSession.emit("session_start", {});
  assert.equal(newMode.current(), null);
});

test("goal pause resume and compaction restoration preserve consumed usage", async () => {
  const runtime = harness();
  const mode = registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  await runtime.registeredCommands
    .get("goal")
    .handler("--tokens 100 work", runtime.context);
  await runtime.emit("before_agent_start", {
    systemPrompt: "base",
    prompt: runtime.messages[0].message.content,
  });
  await runtime.emit("turn_end", {
    message: {
      role: "assistant",
      content: [{ type: "text", text: "working" }],
      usage: { totalTokens: 20 },
    },
    toolResults: [{ role: "toolResult" }],
  });
  await runtime.registeredCommands
    .get("goal")
    .handler("pause", runtime.context);
  const pausedId = mode.current().goalId;
  await runtime.registeredCommands
    .get("goal")
    .handler("resume --tokens 200 --turns 3", runtime.context);
  assert.equal(mode.current().status, "active");
  assert.notEqual(mode.current().goalId, pausedId);
  assert.equal(mode.current().tokenUsage, 20);
  assert.equal(mode.current().automaticResponses, 0);
  assert.equal(mode.current().automaticLimit, 3);

  await runtime.emit("session_tree", {});
  assert.equal(mode.current().tokenUsage, 20);
  assert.equal(mode.current().objective, "work");
});

test("goal adapter rotates stale-turn ownership on direct user intervention", async () => {
  const runtime = harness();
  const mode = registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  await runtime.registeredCommands.get("goal").handler("work", runtime.context);
  const originalId = mode.current().goalId;
  await runtime.emit("input", {
    source: "interactive",
    text: "additional direction",
  });
  assert.notEqual(mode.current().goalId, originalId);
  await assert.rejects(
    runtime.registeredTools
      .find((tool) => tool.name === "goal_complete")
      .execute(
        "call",
        { goal_id: originalId, summary: "Everything was verified" },
        undefined,
        undefined,
        runtime.context,
      ),
    (error) => {
      assert.match(error.message, /completion_stale/);
      assert.match(error.message, new RegExp(mode.current().goalId));
      assert.match(error.message, new RegExp(mode.current().guardEpoch));
      assert.match(error.message, /development_system_goal_status/);
      return true;
    },
  );
  const status = await runtime.registeredTools
    .find((tool) => tool.name === "development_system_goal_status")
    .execute();
  assert.equal(status.details.goal_id, mode.current().goalId);
  assert.equal(status.details.guard_epoch, mode.current().guardEpoch);
  assert.match(status.details.retry, /Retry the terminal tool/);

  await assert.rejects(
    runtime.registeredTools
      .find((tool) => tool.name === "goal_blocked")
      .execute(
        "call",
        {
          goal_id: originalId,
          reason: "The external service requires owner approval",
          evidence:
            "The owner action remained required on attempts 1, 2, and 3",
          repeated_turns: 3,
        },
        undefined,
        undefined,
        runtime.context,
      ),
    (error) => {
      assert.match(error.message, /goal_blocked_stale/);
      assert.match(error.message, new RegExp(mode.current().goalId));
      assert.match(error.message, /refresh_tool/);
      return true;
    },
  );
});

test("goal adapter pauses before an owned call when terminal tools are restricted", async () => {
  const runtime = harness();
  const mode = registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  await runtime.registeredCommands.get("goal").handler("work", runtime.context);
  runtime.pi.getActiveTools = () => ["goal_complete"];
  let aborted = false;
  await runtime.emit(
    "before_agent_start",
    { systemPrompt: "base", prompt: runtime.messages[0].message.content },
    {
      ...runtime.context,
      abort: () => {
        aborted = true;
      },
    },
  );
  assert.equal(aborted, true);
  assert.equal(mode.current().stoppedReason, "terminal-tools-unavailable");
});

test("goal adapter waits through run recovery and stops terminal provider errors only at settlement", async () => {
  const runtime = harness();
  const mode = registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  await runtime.registeredCommands.get("goal").handler("work", runtime.context);
  await runtime.emit("before_agent_start", {
    systemPrompt: "base",
    prompt: runtime.messages[0].message.content,
  });
  await runtime.emit("agent_end", {
    messages: [
      {
        role: "assistant",
        stopReason: "error",
        errorMessage: "provider unavailable",
      },
    ],
  });
  assert.equal(mode.current().status, "active");
  await runtime.emit("agent_settled", {});
  assert.equal(mode.current().status, "paused");
  assert.equal(mode.current().stoppedReason, "terminal-provider-error");
});

test("cleared or replaced ownership cannot dispatch a delayed extension prompt", async () => {
  const runtime = harness();
  registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  await runtime.registeredCommands
    .get("goal")
    .handler("old work", runtime.context);
  const delayed = runtime.messages[0].message.content;
  await runtime.registeredCommands
    .get("goal")
    .handler("clear", runtime.context);
  let aborted = false;
  await runtime.emit(
    "before_agent_start",
    { systemPrompt: "base", prompt: delayed },
    {
      ...runtime.context,
      abort: () => {
        aborted = true;
      },
    },
  );
  assert.equal(aborted, true);
});

test("goal adapter disables all activation on reserved-name collision", async () => {
  const runtime = harness({ commands: ["goal"], tools: ["goal_complete"] });
  const mode = registerGoalMode(runtime.pi);
  await runtime.emit("session_start", {});
  assert.equal(mode.collision, true);
  await runtime.registeredCommands.get("goal").handler("work", runtime.context);
  assert.equal(runtime.messages.length, 0);
});
