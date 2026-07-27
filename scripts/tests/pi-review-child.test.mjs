import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  resolveReviewRoute,
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
  assert.equal(result.result.status, "clean");
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
  await assert.rejects(
    () =>
      runReviewChild({
        assignment: { assignment: "Review", modelRole: "bounded-helper" },
        cwd: failed.root,
        route: resolveReviewRoute("bounded-helper"),
        piBinary: failed.file,
      }),
    /review_child_provider_failed/,
  );
});
