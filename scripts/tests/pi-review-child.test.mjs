import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  resolveReviewRoute,
  reviewFailureResult,
  runReviewChild,
} from "../../plugins/development-system/extensions/development-system/adapters/review-child.ts";

function executable(output, exitCode = 0) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-review-child-"));
  const file = path.join(root, "pi-fixture");
  const events = [
    { type: "agent_start" },
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: output }],
        stopReason: "stop",
      },
    },
    { type: "agent_settled" },
  ];
  const body = events
    .map((event) => `printf '%s\\n' '${JSON.stringify(event)}'`)
    .join("\n");
  fs.writeFileSync(file, `#!/usr/bin/env bash\n${body}\nexit ${exitCode}\n`);
  fs.chmodSync(file, 0o755);
  return { root, file };
}

function eventExecutable(events, exitCode = 0) {
  const body = events
    .map((event) => `printf '%s\\n' '${JSON.stringify(event)}'`)
    .join("\n");
  return scriptExecutable(`${body}\nexit ${exitCode}`);
}

function scriptExecutable(script) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-review-events-"));
  const file = path.join(root, "pi-fixture");
  fs.writeFileSync(file, `#!/usr/bin/env bash\n${script}\n`);
  fs.chmodSync(file, 0o755);
  return { root, file };
}

async function capturedFailure(fixture, options = {}) {
  try {
    await runReviewChild({
      assignment: { assignment: "Review", modelRole: "bounded-helper" },
      cwd: fixture.root,
      route: resolveReviewRoute("bounded-helper"),
      piBinary: fixture.file,
      terminationGraceMs: 25,
      ...options,
    });
  } catch (error) {
    return reviewFailureResult(error);
  }
  assert.fail("expected review child failure");
}

test("review routing maps abstract roles and permits project overrides", () => {
  assert.deepEqual(resolveReviewRoute("strong-reviewer"), {
    provider: "openai-codex",
    model: "gpt-5.6-sol",
    thinking: "high",
  });
  assert.deepEqual(
    resolveReviewRoute("bounded-helper", {
      "bounded-helper": "custom/fast-model",
    }),
    {
      provider: "custom",
      model: "fast-model",
      thinking: "high",
    },
  );
  assert.throws(
    () => resolveReviewRoute("unknown"),
    /review_model_role_unmapped/,
  );
});

test("fresh child returns required role and closure attestation", async () => {
  const fixture = executable('{"status":"clean","findings":[]}');
  const result = await runReviewChild({
    assignment: {
      assignment: "Review the bounded scope",
      modelRole: "strong-reviewer",
    },
    cwd: fixture.root,
    route: resolveReviewRoute("strong-reviewer"),
    piBinary: fixture.file,
  });
  assert.equal(result.status, "completed");
  assert.equal(result.result.status, "clean");
  assert.equal(result.lifecycle.state, "completed");
  assert.equal(result.lifecycle.processStarted, true);
  assert.equal(result.progress.state, "settled");
  assert.deepEqual(result.attestation, {
    model_role: "strong-reviewer",
    fresh_context: true,
    closed_after_result: true,
  });
});

test("progress reports bounded lifecycle and tool metadata without child content", async () => {
  const privatePath = "/private/client/secret.txt";
  const fixture = eventExecutable([
    { type: "agent_start" },
    { type: "turn_start", turnIndex: 0 },
    {
      type: "tool_execution_start",
      toolCallId: "tool-1",
      toolName: "read",
      args: { path: privatePath },
    },
    {
      type: "tool_execution_start",
      toolCallId: "tool-2",
      toolName: "client-secret-path",
      args: { value: "private tool argument" },
    },
    {
      type: "tool_execution_end",
      toolCallId: "unknown-tool",
      toolName: "read",
      isError: true,
    },
    {
      type: "tool_execution_end",
      toolCallId: "tool-1",
      toolName: "read",
      result: {
        content: [{ type: "text", text: "private transcript content" }],
      },
      isError: false,
    },
    {
      type: "tool_execution_end",
      toolCallId: "tool-2",
      toolName: "client-secret-path",
      isError: false,
    },
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: '{"status":"clean","findings":[]}' }],
        stopReason: "stop",
      },
    },
    { type: "agent_settled" },
  ]);
  const updates = [];
  const result = await runReviewChild({
    assignment: {
      assignment: "Review the bounded scope",
      modelRole: "strong-reviewer",
    },
    cwd: fixture.root,
    route: resolveReviewRoute("strong-reviewer"),
    piBinary: fixture.file,
    progressThrottleMs: 0,
    onProgress: (progress) => updates.push(progress),
  });

  assert.ok(updates.some((update) => update.state === "tool-running"));
  assert.ok(updates.some((update) => update.currentTool === "read"));
  assert.ok(
    updates.some(
      (update) =>
        update.state === "tool-running" &&
        update.currentTool === "extension-tool" &&
        update.activeToolCount === 1,
    ),
  );
  assert.equal(result.progress.turns, 1);
  assert.equal(result.progress.toolCalls, 2);
  assert.equal(result.progress.toolErrors, 0);
  assert.ok(
    updates.some(
      (update) =>
        update.activeToolCount === 2 &&
        update.recentEvents.at(-1)?.message ===
          "Ignored unmatched completion for read; running extension-tool",
    ),
  );
  assert.ok(result.progress.recentEvents.length <= 20);
  const observable = JSON.stringify({ updates, progress: result.progress });
  assert.equal(observable.includes(privatePath), false);
  assert.equal(observable.includes("private transcript content"), false);
  assert.equal(observable.includes("client-secret-path"), false);
  assert.equal(observable.includes("private tool argument"), false);
});

test("progress heartbeats advance elapsed time between child events", async () => {
  const start = JSON.stringify({ type: "agent_start" });
  const response = JSON.stringify({
    type: "message_end",
    message: {
      role: "assistant",
      content: [{ type: "text", text: '{"status":"clean"}' }],
      stopReason: "stop",
    },
  });
  const settled = JSON.stringify({ type: "agent_settled" });
  const fixture = scriptExecutable(
    `printf '%s\\n' '${start}'\nsleep 0.08\nprintf '%s\\n' '${response}' '${settled}'`,
  );
  const updates = [];
  await runReviewChild({
    assignment: { assignment: "Review", modelRole: "bounded-helper" },
    cwd: fixture.root,
    route: resolveReviewRoute("bounded-helper"),
    piBinary: fixture.file,
    progressIntervalMs: 10,
    progressThrottleMs: 0,
    onProgress: (progress) => updates.push(progress),
  });

  assert.ok(
    updates.some(
      (update) => update.state === "running" && update.elapsedMs >= 20,
    ),
  );
});

test("progress history remains bounded while preserving aggregate counts", async () => {
  const toolEvents = Array.from({ length: 40 }, (_, index) => ({
    type: "tool_execution_start",
    toolCallId: `tool-${index}`,
    toolName: "read",
    args: { path: `/private/${index}` },
  }));
  const fixture = eventExecutable([
    { type: "agent_start" },
    ...toolEvents,
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: '{"status":"clean"}' }],
        stopReason: "stop",
      },
    },
    { type: "agent_settled" },
  ]);
  const updates = [];
  const result = await runReviewChild({
    assignment: { assignment: "Review", modelRole: "bounded-helper" },
    cwd: fixture.root,
    route: resolveReviewRoute("bounded-helper"),
    piBinary: fixture.file,
    progressThrottleMs: 0,
    onProgress: (progress) => updates.push(progress),
  });

  assert.equal(result.progress.toolCalls, 40);
  assert.equal(
    Math.max(...updates.map((progress) => progress.activeToolCount)),
    40,
  );
  assert.equal(result.progress.recentEvents.length, 20);
  assert.ok(result.progress.droppedEvents > 0);
});

test("noisy recognized events are coalesced before parent updates", async () => {
  const toolEvents = Array.from({ length: 200 }, (_, index) => ({
    type: "tool_execution_start",
    toolCallId: `tool-${index}`,
    toolName: "read",
  }));
  const fixture = eventExecutable([
    { type: "agent_start" },
    ...toolEvents,
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: '{"status":"clean"}' }],
        stopReason: "stop",
      },
    },
    { type: "agent_settled" },
  ]);
  const updates = [];
  const result = await runReviewChild({
    assignment: { assignment: "Review", modelRole: "bounded-helper" },
    cwd: fixture.root,
    route: resolveReviewRoute("bounded-helper"),
    piBinary: fixture.file,
    progressThrottleMs: 10_000,
    onProgress: (progress) => updates.push(progress),
  });

  assert.equal(result.progress.toolCalls, 200);
  assert.equal(updates.length, 2);
  assert.equal(updates.at(-1).state, "settled");
});

test("oversized structured results fail without exposing child content", async () => {
  const secretMarker = "private-result-marker";
  const fixture = executable(
    JSON.stringify({ value: `${secretMarker}${"x".repeat(52 * 1024)}` }),
  );
  let limited;
  try {
    await runReviewChild({
      assignment: { assignment: "Review", modelRole: "bounded-helper" },
      cwd: fixture.root,
      route: resolveReviewRoute("bounded-helper"),
      piBinary: fixture.file,
    });
  } catch (error) {
    limited = reviewFailureResult(error);
  }

  assert.equal(limited.code, "development_system.review_child_output_limit");
  assert.equal(limited.reason, "output-limit");
  assert.equal(limited.lifecycle.state, "output-limited");
  assert.equal(JSON.stringify(limited).includes(secretMarker), false);
});

test("oversized assistant output terminates a still-running child immediately", async () => {
  const event = JSON.stringify({
    type: "message_end",
    message: {
      role: "assistant",
      content: [
        {
          type: "text",
          text: JSON.stringify({ value: "x".repeat(52 * 1024) }),
        },
      ],
      stopReason: "stop",
    },
  });
  const fixture = scriptExecutable(`printf '%s\\n' '${event}'\nsleep 30`);
  const started = Date.now();
  const limited = await capturedFailure(fixture, { timeoutMs: 5_000 });

  assert.equal(limited.code, "development_system.review_child_output_limit");
  assert.ok(Date.now() - started < 1_000);
  assert.equal(limited.lifecycle.terminationRequested, true);
});

test("malformed JSONL and every protocol stream bound fail closed", async () => {
  const malformed = await capturedFailure(
    scriptExecutable(`printf '%s\\n' 'not-json'`),
  );
  assert.equal(
    malformed.code,
    "development_system.review_child_result_malformed",
  );
  assert.equal(malformed.reason, "malformed-result");

  const longLine = JSON.stringify({ type: "session", padding: "x".repeat(80) });
  const lineLimited = await capturedFailure(
    scriptExecutable(`printf '%s\\n' '${longLine}'`),
    { protocolLineLimitBytes: 32, protocolStreamLimitBytes: 1_024 },
  );
  assert.equal(
    lineLimited.code,
    "development_system.review_child_output_limit",
  );

  const eventLine = JSON.stringify({ type: "session" });
  const streamLimited = await capturedFailure(
    scriptExecutable(
      Array.from({ length: 8 }, () => `printf '%s\\n' '${eventLine}'`).join(
        "\n",
      ),
    ),
    { protocolLineLimitBytes: 128, protocolStreamLimitBytes: 64 },
  );
  assert.equal(
    streamLimited.code,
    "development_system.review_child_output_limit",
  );

  const stderrLimited = await capturedFailure(
    scriptExecutable(`printf '%s' 'private-stderr-content' >&2`),
    { stderrLimitBytes: 8 },
  );
  assert.equal(
    stderrLimited.code,
    "development_system.review_child_output_limit",
  );
  assert.equal(
    JSON.stringify(stderrLimited).includes("private-stderr-content"),
    false,
  );
});

test("an empty final response cannot reuse an intermediate assistant result", async () => {
  const fixture = eventExecutable([
    { type: "agent_start" },
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: '{"status":"clean"}' }],
        stopReason: "toolUse",
      },
    },
    {
      type: "message_end",
      message: { role: "assistant", content: [], stopReason: "stop" },
    },
    { type: "agent_settled" },
  ]);
  const malformed = await capturedFailure(fixture);

  assert.equal(
    malformed.code,
    "development_system.review_child_result_malformed",
  );
});

test("empty provider error messages retain provider-failure classification", async () => {
  const fixture = eventExecutable([
    { type: "agent_start" },
    {
      type: "message_end",
      message: { role: "assistant", content: [], stopReason: "error" },
    },
    { type: "agent_settled" },
  ]);
  let providerFailure;
  try {
    await runReviewChild({
      assignment: { assignment: "Review", modelRole: "bounded-helper" },
      cwd: fixture.root,
      route: resolveReviewRoute("bounded-helper"),
      piBinary: fixture.file,
    });
  } catch (error) {
    providerFailure = reviewFailureResult(error);
  }

  assert.equal(
    providerFailure.code,
    "development_system.review_child_provider_failed",
  );
  assert.equal(providerFailure.reason, "provider-exit");
});

test("provider and malformed results remain unresolved failures", async () => {
  const malformed = executable("not-json");
  await assert.rejects(
    () =>
      runReviewChild({
        assignment: { assignment: "Review", modelRole: "bounded-helper" },
        cwd: malformed.root,
        route: resolveReviewRoute("bounded-helper"),
        piBinary: malformed.file,
      }),
    /review_child_result_malformed/,
  );
  const failed = executable('{"status":"clean"}', 7);
  let providerFailure;
  try {
    await runReviewChild({
      assignment: { assignment: "Review", modelRole: "bounded-helper" },
      cwd: failed.root,
      route: resolveReviewRoute("bounded-helper"),
      piBinary: failed.file,
    });
  } catch (error) {
    providerFailure = reviewFailureResult(error);
  }
  assert.equal(
    providerFailure.code,
    "development_system.review_child_provider_failed",
  );
  assert.equal(providerFailure.reason, "provider-exit");
  assert.equal(providerFailure.lifecycle.exitCode, 7);
  assert.equal(
    JSON.stringify(providerFailure).includes('status":"clean'),
    false,
  );
});

test("cancelled and timed-out children expose structured lifecycle diagnostics", async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-review-cancel-"));
  const file = path.join(root, "pi-fixture");
  fs.writeFileSync(
    file,
    "#!/usr/bin/env bash\nprintf 'private-provider-detail' >&2\nsleep 30\n",
  );
  fs.chmodSync(file, 0o755);
  const controller = new AbortController();
  const pending = runReviewChild({
    assignment: { assignment: "Review", modelRole: "bounded-helper" },
    cwd: root,
    route: resolveReviewRoute("bounded-helper"),
    piBinary: file,
    signal: controller.signal,
  });
  setTimeout(() => controller.abort(), 25);
  let cancelled;
  try {
    await pending;
  } catch (error) {
    cancelled = reviewFailureResult(error);
  }
  assert.equal(cancelled.status, "failed");
  assert.equal(cancelled.code, "development_system.review_child_cancelled");
  assert.equal(cancelled.reason, "parent-abort");
  assert.equal(cancelled.lifecycle.state, "cancelled");
  assert.equal(cancelled.lifecycle.terminationRequested, true);
  assert.ok(cancelled.lifecycle.stderrBytes > 0);
  assert.equal(
    JSON.stringify(cancelled).includes("private-provider-detail"),
    false,
  );
  assert.match(cancelled.retry, /rerun the assignment/);

  let timedOut;
  try {
    await runReviewChild({
      assignment: { assignment: "Review", modelRole: "bounded-helper" },
      cwd: root,
      route: resolveReviewRoute("bounded-helper"),
      piBinary: file,
      timeoutMs: 25,
    });
  } catch (error) {
    timedOut = reviewFailureResult(error);
  }
  assert.equal(timedOut.code, "development_system.review_child_timeout");
  assert.equal(timedOut.reason, "timeout");
  assert.equal(timedOut.lifecycle.state, "timed-out");
});

test("successful children close background descendants before attesting closure", async () => {
  const events = [
    { type: "agent_start" },
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: '{"status":"clean"}' }],
        stopReason: "stop",
      },
    },
    { type: "agent_settled" },
  ];
  const fixture = scriptExecutable(
    `sleep 30 &\necho $! > descendant.pid\n${events
      .map((event) => `printf '%s\\n' '${JSON.stringify(event)}'`)
      .join("\n")}`,
  );
  const result = await runReviewChild({
    assignment: { assignment: "Review", modelRole: "bounded-helper" },
    cwd: fixture.root,
    route: resolveReviewRoute("bounded-helper"),
    piBinary: fixture.file,
    terminationGraceMs: 50,
  });
  const descendantPid = Number(
    fs.readFileSync(path.join(fixture.root, "descendant.pid"), "utf8"),
  );

  assert.equal(result.status, "completed");
  assert.equal(result.lifecycle.terminationRequested, true);
  assert.throws(() => process.kill(descendantPid, 0));
});

test("cancellation escalates and waits for a TERM-resistant child to close", async () => {
  const fixture = scriptExecutable(
    `trap '' TERM\nprintf '%s\\n' '${JSON.stringify({ type: "agent_start" })}'\nwhile true; do sleep 30; done`,
  );
  const controller = new AbortController();
  const pending = runReviewChild({
    assignment: { assignment: "Review", modelRole: "bounded-helper" },
    cwd: fixture.root,
    route: resolveReviewRoute("bounded-helper"),
    piBinary: fixture.file,
    signal: controller.signal,
    terminationGraceMs: 25,
  });
  setTimeout(() => controller.abort(), 25);
  let cancelled;
  try {
    await pending;
  } catch (error) {
    cancelled = reviewFailureResult(error);
  }

  assert.equal(cancelled.code, "development_system.review_child_cancelled");
  assert.equal(cancelled.lifecycle.terminationRequested, true);
  assert.equal(cancelled.lifecycle.signal, "SIGKILL");
  assert.ok(cancelled.lifecycle.exitCode === null);
});
