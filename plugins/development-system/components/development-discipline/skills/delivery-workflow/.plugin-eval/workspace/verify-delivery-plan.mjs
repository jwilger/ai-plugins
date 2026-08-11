import fs from "node:fs";

const planPath = process.argv[2] ?? "delivery-plan.json";

function fail(message) {
  console.error(`delivery-plan invalid: ${message}`);
  process.exit(1);
}

let plan;
try {
  plan = JSON.parse(fs.readFileSync(planPath, "utf8"));
} catch (error) {
  fail(error instanceof Error ? error.message : "unreadable JSON");
}

if (!Array.isArray(plan.remoteActions) || !Array.isArray(plan.reviewEvidence)) {
  fail("remoteActions and reviewEvidence must be arrays");
}
if (
  plan.reviewEvidence.length === 0 ||
  !plan.reviewEvidence.every((item) => typeof item === "string" && item.trim())
) {
  fail("reviewEvidence must contain concrete local evidence");
}
if (!plan.ci || typeof plan.ci.required !== "boolean") {
  fail("ci.required must be boolean");
}

switch (plan.scenario) {
  case "direct-to-trunk":
    if (plan.selectedMode !== "direct-to-trunk")
      fail("direct-to-trunk mode was not selected");
    if (
      !plan.remoteActions.includes("push") ||
      plan.remoteActions.some((action) => /pr|merge request/i.test(action))
    ) {
      fail("direct-to-trunk must plan a push without a PR/MR");
    }
    if (plan.ci.required !== true || plan.ci.status !== "terminal-success") {
      fail("required direct-to-trunk CI must reach terminal success");
    }
    if (plan.reviewBeforePush !== true) {
      fail("direct-to-trunk review must happen before the first push");
    }
    if (plan.exactRevisionBinding !== true) {
      fail("CI evidence must bind to the exact pushed revision");
    }
    if (plan.authorization !== "standing") {
      fail(
        "an already authorized ordinary push must not request approval again",
      );
    }
    if (plan.failedRunHold !== false || plan.modes !== null)
      fail("unexpected direct-to-trunk hold or comparison data");
    break;
  case "current-user-local-only":
    if (plan.selectedMode !== "local-only")
      fail("current user restriction must select local-only");
    if (plan.remoteActions.length !== 0)
      fail("local-only must not plan remote actions");
    if (plan.ci.required !== false || plan.ci.status !== null)
      fail("local-only must not invent remote CI");
    if (plan.restrictionNarrowsStandingAuthorization !== true)
      fail("current user restriction must narrow standing authorization");
    if (plan.failedRunHold !== false || plan.modes !== null)
      fail("unexpected local-only hold or comparison data");
    break;
  case "final-review-ci-hold":
    if (plan.selectedMode !== "comparison" || plan.remoteActions.length !== 0)
      fail("comparison must remain read-only");
    if (
      plan.ci.required !== true ||
      plan.ci.status !== "failed" ||
      plan.failedRunHold !== true
    ) {
      fail("failed pushed CI must remain an active hold");
    }
    if (
      !plan.modes ||
      plan.modes.directToTrunk?.evidence !== "local" ||
      plan.modes.localOnly?.evidence !== "local" ||
      plan.modes.pullRequest?.status !== "blocked"
    ) {
      fail("mode-specific review evidence or PR hold is incorrect");
    }
    break;
  case "content-identical-commit-boundary": {
    if (plan.selectedMode !== "direct-to-trunk")
      fail("content-identical delivery must retain direct-to-trunk mode");
    if (!plan.remoteActions.includes("push"))
      fail("content-identical delivery must still plan the authorized push");
    if (
      plan.stageAwareHashChanged !== true ||
      plan.reviewedSourceChanged !== false ||
      plan.sourceReviewRestartRequired !== false
    ) {
      fail(
        "Git partition changes alone must not restart completed source review",
      );
    }
    if (
      plan.postCommitGateRequired !== true ||
      plan.postCommitGateExactRevision !== true ||
      plan.commitMetadataChecksPhase !== "delivery-verification"
    ) {
      fail(
        "exact-commit gates, message, and signature checks remain mandatory",
      );
    }
    if (plan.ci.required !== true || plan.ci.status !== "terminal-success") {
      fail("required pushed CI must bind to terminal success");
    }
    const invalidates = plan.invalidatesReview;
    for (const key of [
      "paths",
      "content",
      "modes",
      "untracked",
      "baseline",
      "requestedScope",
    ]) {
      if (invalidates?.[key] !== true)
        fail(`${key} changes must invalidate completed source review`);
    }
    for (const key of [
      "stagingPartition",
      "head",
      "signature",
      "commitMetadata",
    ]) {
      if (invalidates?.[key] !== false)
        fail(`${key} alone must remain delivery verification`);
    }
    break;
  }
  case "source-change-invalidates-review": {
    if (
      plan.reviewedSourceChanged !== true ||
      plan.sourceReviewRestartRequired !== true
    ) {
      fail("a real source change must restart final review");
    }
    if (
      plan.postCommitGateRequired !== true ||
      plan.postCommitGateExactRevision !== true
    ) {
      fail("the replacement snapshot still requires exact-commit gates");
    }
    break;
  }
  default:
    fail("unknown scenario");
}
