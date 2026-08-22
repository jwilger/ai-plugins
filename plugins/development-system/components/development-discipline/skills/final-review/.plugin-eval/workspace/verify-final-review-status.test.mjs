import assert from "node:assert/strict";
import test from "node:test";
import { validateFinalReviewStatus } from "./final-review-status-contract.mjs";

const scenarios = [
  "coordinator-unavailable",
  "one-pass-request",
  "budget-pressure",
];

const validStatus = (scenario) => ({
  scenario,
  reviewComplete: false,
  enforcementAvailable: false,
  deliveryAllowed: false,
  requiredCleanIterations: 3,
  cleanIterationsAccepted: 0,
  shortcutAccepted: false,
  manualReviewPermitted: true,
  prohibitedActions: ["commit", "push", "delivery", "readiness-claim"],
  reason:
    scenario === "coordinator-unavailable"
      ? "The MCP enforcement coordinator is unavailable."
      : scenario === "one-pass-request"
        ? "A one-pass shortcut cannot replace three clean passes."
        : "Budget pressure cannot skip the three remaining clean-pass gate.",
});

for (const scenario of scenarios) {
  test(`${scenario} accepts its own bound artifact`, () => {
    assert.deepEqual(
      validateFinalReviewStatus(validStatus(scenario), scenario),
      [],
    );
  });

  for (const differentScenario of scenarios.filter(
    (item) => item !== scenario,
  )) {
    test(`${scenario} rejects a ${differentScenario} artifact`, () => {
      assert.ok(
        validateFinalReviewStatus(
          validStatus(differentScenario),
          scenario,
        ).some((error) => error.startsWith("scenario must be")),
      );
    });
  }
}

test("semantic prohibition phrases cover every protected capability", () => {
  const status = validStatus("coordinator-unavailable");
  status.prohibitedActions = [
    "perform Git actions",
    "perform remote actions",
    "create a pull request",
    "merge",
    "claim final-review completion",
    "claim readiness",
  ];
  assert.deepEqual(
    validateFinalReviewStatus(status, "coordinator-unavailable"),
    [],
  );
});

test("natural-language gerunds cover commit and push prohibitions", () => {
  const status = validStatus("budget-pressure");
  status.prohibitedActions = [
    "claiming review completion or readiness",
    "committing changes",
    "pushing changes",
    "creating or merging a pull or merge request",
  ];
  assert.deepEqual(validateFinalReviewStatus(status, "budget-pressure"), []);
});

test("scenario evidence may be explicit in the prohibition list", () => {
  const status = validStatus("one-pass-request");
  status.reason =
    "The enforcement service is unavailable, so no clean iteration is accepted and delivery is unauthorized.";
  status.prohibitedActions.push("Accept a one-pass shortcut");
  assert.deepEqual(validateFinalReviewStatus(status, "one-pass-request"), []);
});

test("missing a protected capability remains invalid", () => {
  const status = validStatus("coordinator-unavailable");
  status.prohibitedActions = ["commit", "push", "delivery"];
  assert.ok(
    validateFinalReviewStatus(status, "coordinator-unavailable").includes(
      "prohibitedActions must prohibit readiness-claim",
    ),
  );
});
