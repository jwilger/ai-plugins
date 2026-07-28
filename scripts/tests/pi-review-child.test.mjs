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
  fs.writeFileSync(
    file,
    `#!/usr/bin/env bash\nprintf '%s\\n' '${output}'\nexit ${exitCode}\n`,
  );
  fs.chmodSync(file, 0o755);
  return { root, file };
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
  assert.deepEqual(result.attestation, {
    model_role: "strong-reviewer",
    fresh_context: true,
    closed_after_result: true,
  });
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
  assert.equal(providerFailure.code, "development_system.review_child_provider_failed");
  assert.equal(providerFailure.reason, "provider-exit");
  assert.equal(providerFailure.lifecycle.exitCode, 7);
  assert.equal(JSON.stringify(providerFailure).includes("status\":\"clean"), false);
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
  assert.equal(JSON.stringify(cancelled).includes("private-provider-detail"), false);
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
