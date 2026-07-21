import fs from "node:fs";

const planPath = process.argv[2] ?? "workflow-plan.json";

function fail(message) {
  console.error(`workflow-plan invalid: ${message}`);
  process.exit(1);
}

function hasProhibition(fragment) {
  const normalizedFragment = fragment.replaceAll("-", " ");
  return plan.prohibitedActions.some((action) =>
    String(action).replaceAll("-", " ").includes(normalizedFragment),
  );
}

let plan;
try {
  plan = JSON.parse(fs.readFileSync(planPath, "utf8"));
} catch (error) {
  fail(error instanceof Error ? error.message : "unreadable JSON");
}

for (const key of ["orderedPhases", "specialists", "prohibitedActions"]) {
  if (!Array.isArray(plan[key])) fail(`${key} must be an array`);
}
for (const key of [
  "inspectState",
  "deliverySelectedBeforePreservation",
  "exactRevisionBinding",
  "ciFailureHold",
  "mutationsPlanned",
]) {
  if (typeof plan[key] !== "boolean") fail(`${key} must be boolean`);
}
if (plan.inspectState !== true)
  fail("repository state must be inspected first");

switch (plan.scenario) {
  case "implementation-routing": {
    const required = [
      "inspect-state",
      "change-preflight",
      "delivery-selection",
      "test-driven-development",
      "verification-before-completion",
      "final-review",
      "final-delivery",
      "exact-revision-ci",
    ];
    if (JSON.stringify(plan.orderedPhases) !== JSON.stringify(required)) {
      fail("implementation phases are missing or out of order");
    }
    if (
      !["inspect-state", "implementation"].includes(plan.activePhase) ||
      plan.deliverySelectedBeforePreservation !== true ||
      plan.exactRevisionBinding !== true ||
      plan.ciFailureHold !== false ||
      plan.mutationsPlanned !== false
    ) {
      fail("implementation routing invariants are incorrect");
    }
    const requiredSpecialists = [
      "change-preflight",
      "delivery-workflow",
      "test-driven-development",
      "verification-before-completion",
      "final-review",
      "rationale-commit-messages",
    ];
    if (
      plan.specialists.length !== requiredSpecialists.length ||
      !requiredSpecialists.every((skill) => plan.specialists.includes(skill))
    ) {
      fail("implementation specialists must contain the exact required set");
    }
    for (const action of ["pull-request", "merge"]) {
      if (!plan.prohibitedActions.includes(action)) {
        fail(`direct-to-main plan must prohibit ${action}`);
      }
    }
    if (plan.resumeWhen !== "terminal-success") {
      fail("required exact-revision CI must reach terminal success");
    }
    break;
  }
  case "pushed-ci-hold":
    if (
      plan.activePhase !== "ci-failure-follow-up" ||
      JSON.stringify(plan.orderedPhases) !==
        JSON.stringify(["inspect-state", "ci-failure-follow-up"]) ||
      JSON.stringify(plan.specialists) !==
        JSON.stringify(["ci-failure-follow-up"]) ||
      plan.ciFailureHold !== true ||
      plan.exactRevisionBinding !== true ||
      plan.mutationsPlanned !== false ||
      plan.resumeWhen !== "replacement-terminal-success"
    ) {
      fail("pushed CI hold invariants are incorrect");
    }
    for (const action of ["unrelated-work", "noncausal-push"]) {
      if (!plan.prohibitedActions.includes(action))
        fail(`missing prohibition ${action}`);
    }
    break;
  case "review-only-boundary":
    if (
      !["answer-review", "answer-or-review-only", "review-only"].includes(
        plan.activePhase,
      ) ||
      plan.orderedPhases.length < 2 ||
      !plan.orderedPhases.some((phase) => String(phase).includes("review")) ||
      plan.orderedPhases.some((phase) =>
        [
          "change-preflight",
          "test-driven",
          "implementation",
          "verification",
          "final-review",
          "delivery",
          "commit",
          "push",
          "pull-request",
          "merge",
          "ci-failure",
        ].some((fragment) => String(phase).includes(fragment)),
      ) ||
      plan.specialists.some((skill) =>
        [
          "change-preflight",
          "test-driven-development",
          "verification-before-completion",
          "delivery-workflow",
          "final-review",
          "rationale-commit-messages",
          "ci-failure-follow-up",
          "receiving-code-review",
          "babysit-pr",
        ].includes(skill),
      ) ||
      plan.deliverySelectedBeforePreservation !== false ||
      plan.exactRevisionBinding !== false ||
      plan.ciFailureHold !== false ||
      plan.mutationsPlanned !== false ||
      plan.resumeWhen.length === 0
    ) {
      fail("review-only boundary invariants are incorrect");
    }
    for (const action of ["edit", "ticket", "commit", "push", "pull-request"]) {
      if (!hasProhibition(action))
        fail(`missing prohibition ${action}`);
    }
    break;
  default:
    fail("unknown scenario");
}
