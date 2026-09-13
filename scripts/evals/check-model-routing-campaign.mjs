#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const expectedTaskFamilies = [
  "mechanical-assistance",
  "implementation-and-tests",
  "review",
  "diagnosis-and-correction",
  "architecture-and-advice",
  "orchestration",
];
const expectedCaseKinds = [
  "nominal",
  "boundary",
  "expected-error-or-refusal",
  "partial-credit",
  "adversarial",
  "regression",
];
const expectedModels = [
  "gpt-5.6-luna",
  "gpt-5.6-terra",
  "gpt-5.6-sol",
  "gpt-6-astra",
];
const expectedEfforts = ["low", "medium", "high", "xhigh", "max"];
const expectedRubricFamilies = [
  "instruction-adherence",
  "engineering-and-architecture",
  "review-findings",
  "orchestration-and-advice",
];
const expectedWorkflowCosts = [
  "orchestration",
  "context-replay",
  "workers",
  "operational-review",
  "corrections",
  "retries",
  "escalation",
];

function fail(message) {
  throw new Error(`model-routing campaign invalid: ${message}`);
}

function exactArray(value, expected, label) {
  if (
    !Array.isArray(value) ||
    JSON.stringify(value) !== JSON.stringify(expected)
  ) {
    fail(`${label} must equal ${JSON.stringify(expected)}`);
  }
}

function exactKeys(value, expected, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail(`${label} must be an object`);
  }
  exactArray(Object.keys(value).sort(), [...expected].sort(), `${label} keys`);
}

function positiveInteger(value, expected, label) {
  if (!Number.isInteger(value) || value !== expected) {
    fail(`${label} must equal ${expected}`);
  }
}

function validate(document) {
  exactKeys(
    document,
    [
      "schema_version",
      "campaign_id",
      "status",
      "behavior_claim",
      "unit_of_analysis",
      "task_families",
      "case_kinds",
      "candidates",
      "sampling",
      "promotion",
      "judge",
      "cost",
      "execution",
      "artifacts",
    ],
    "root",
  );
  if (document.schema_version !== 1) fail("schema_version must equal 1");
  if (document.status !== "measurement") fail("status must equal measurement");
  if (document.unit_of_analysis !== "accepted task trajectory") {
    fail("unit_of_analysis must equal accepted task trajectory");
  }
  exactArray(document.task_families, expectedTaskFamilies, "task_families");
  exactArray(document.case_kinds, expectedCaseKinds, "case_kinds");

  exactArray(document.candidates?.models, expectedModels, "candidates.models");
  exactArray(
    document.candidates?.efforts,
    expectedEfforts,
    "candidates.efforts",
  );
  exactArray(
    document.candidates?.mechanical_additional_efforts,
    ["none"],
    "candidates.mechanical_additional_efforts",
  );
  if (document.candidates?.unsupported_combinations_are_results !== true) {
    fail("candidates.unsupported_combinations_are_results must be true");
  }
  if (document.candidates?.service_tier !== "standard") {
    fail("candidates.service_tier must equal standard");
  }

  positiveInteger(
    document.sampling?.screening?.distinct_cases_per_task_family,
    8,
    "sampling.screening.distinct_cases_per_task_family",
  );
  positiveInteger(
    document.sampling?.confirmation?.held_out_distinct_cases_per_task_family,
    160,
    "sampling.confirmation.held_out_distinct_cases_per_task_family",
  );
  positiveInteger(
    document.sampling?.reliability?.difficult_cases_per_task_family,
    8,
    "sampling.reliability.difficult_cases_per_task_family",
  );
  positiveInteger(
    document.sampling?.reliability?.samples_per_case,
    3,
    "sampling.reliability.samples_per_case",
  );
  positiveInteger(
    document.sampling?.workflow?.distinct_scenarios,
    24,
    "sampling.workflow.distinct_scenarios",
  );
  positiveInteger(
    document.sampling?.workflow?.samples_per_scenario,
    3,
    "sampling.workflow.samples_per_scenario",
  );
  exactArray(
    document.sampling?.workflow?.policies,
    ["incumbent", "10kr-inspired", "selected"],
    "sampling.workflow.policies",
  );

  const bound = document.promotion?.quality?.candidate_only_failure_upper_bound;
  if (
    !bound ||
    bound.confidence !== 0.95 ||
    bound.maximum !== 0.02 ||
    bound.sidedness !== "one-sided"
  ) {
    fail(
      "promotion.quality.candidate_only_failure_upper_bound must be a one-sided 95% bound below 0.02",
    );
  }
  if (
    document.promotion.quality.critical_regressions_allowed !== 0 ||
    document.promotion.quality.accepted_task_success !==
      "not-lower-than-incumbent" ||
    document.promotion.quality.inconclusive_result !== "retain-incumbent"
  ) {
    fail("promotion.quality does not preserve the conservative quality gate");
  }
  if (
    document.promotion?.efficiency?.minimum_subscription_savings !== 0.25 ||
    document.promotion.efficiency.maximum_completion_time_increase !== 0.25
  ) {
    fail("promotion.efficiency does not preserve the approved quota tradeoff");
  }

  exactArray(
    document.judge?.rubric_families,
    expectedRubricFamilies,
    "judge.rubric_families",
  );
  if (
    document.judge?.isolation?.plugins !== "none" ||
    document.judge.isolation.tools !== "none" ||
    document.judge.isolation.workspace !== "none" ||
    document.judge.isolation.candidate_identity !== "blinded"
  ) {
    fail(
      "judge.isolation must remove plugins, tools, workspace, and model identity",
    );
  }
  exactArray(
    document.judge?.reference?.semantic_assessors,
    ["gpt-6-astra/high", "gpt-5.6-sol/high"],
    "judge.reference.semantic_assessors",
  );
  if (
    document.judge.reference.deterministic_witnesses_first !== true ||
    document.judge.reference.disagreement !== "unresolved" ||
    document.judge.calibration.screening_examples_per_rubric_family !== 24 ||
    document.judge.calibration.held_out_examples_per_rubric_family !== 160 ||
    document.judge.calibration.borderline_samples !== 3 ||
    document.judge.calibration.minimum_agreement !== 0.95 ||
    document.judge.calibration.critical_false_accepts_allowed !== 0
  ) {
    fail("judge calibration does not preserve the approved reference contract");
  }

  exactArray(
    document.cost?.accounting?.workflow,
    expectedWorkflowCosts,
    "cost.accounting.workflow",
  );
  if (document.cost?.accounting?.benchmark_judges !== "reported-separately") {
    fail("cost.accounting.benchmark_judges must equal reported-separately");
  }
  if (document.cost.accounting.missing_usage !== "unknown") {
    fail("cost.accounting.missing_usage must equal unknown");
  }
  if (
    document.execution?.max_concurrency !== 2 ||
    document.execution.cache !== "disabled" ||
    document.execution.resumable !== true ||
    document.execution.purchase_credits !== false
  ) {
    fail(
      "execution must be finite, resumable, cache-disabled, and concurrency two",
    );
  }

  for (const [name, artifactPath] of Object.entries(document.artifacts ?? {})) {
    if (
      typeof artifactPath !== "string" ||
      path.isAbsolute(artifactPath) ||
      artifactPath.split("/").includes("..")
    ) {
      fail(`artifacts.${name} must be a repository-relative path`);
    }
  }
}

function main() {
  if (process.argv.length !== 3) {
    console.error("usage: check-model-routing-campaign.mjs CAMPAIGN_JSON");
    process.exitCode = 2;
    return;
  }
  try {
    const document = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
    validate(document);
    console.log("model-routing campaign contract valid");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 2;
  }
}

main();
