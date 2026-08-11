import assert from "node:assert/strict";
import test from "node:test";

import { sourceViolations } from "./check-lint-policy.mjs";

test("accepts narrowly scoped reasoned Clippy expectations", () => {
  assert.deepEqual(
    sourceViolations(`#[expect(
      clippy::implicit_return,
      reason = "expression form is clearer"
    )]`),
    [],
  );
});

test("rejects direct and multiline Clippy allows", () => {
  assert.equal(sourceViolations("#[allow(clippy::panic)]").length, 1);
  assert.equal(
    sourceViolations(`#[cfg_attr(test, allow(
      clippy::panic
    ))]`).length,
    1,
  );
});

test("rejects non-Clippy and unreasoned expectations", () => {
  assert.equal(
    sourceViolations('#[expect(dead_code, reason = "temporary")]').length,
    1,
  );
  assert.equal(sourceViolations("#[expect(clippy::panic)]").length, 1);
  assert.equal(
    sourceViolations(
      '#[cfg_attr(test, expect(dead_code, reason = "not clippy"))]',
    ).length,
    1,
  );
  assert.equal(
    sourceViolations("#[cfg_attr(test, expect(clippy::panic))]").length,
    1,
  );
});
