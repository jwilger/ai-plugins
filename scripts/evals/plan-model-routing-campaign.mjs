#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

function fail(message) {
  throw new Error(`model-routing planner: ${message}`);
}

function parseArguments(argv) {
  if (argv.length < 1) {
    fail(
      "usage: plan-model-routing-campaign.mjs CAMPAIGN --phase screening --output PLAN [--resume]",
    );
  }

  const campaignPath = path.resolve(argv[0]);
  let phase;
  let outputPath;
  let resume = false;

  for (let index = 1; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--resume") {
      if (resume) fail("--resume may be specified only once");
      resume = true;
      continue;
    }
    if (argument !== "--phase" && argument !== "--output") {
      fail(`unknown argument: ${argument}`);
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      fail(`${argument} requires a value`);
    }
    if (argument === "--phase") {
      if (phase) fail("--phase may be specified only once");
      phase = value;
    } else {
      if (outputPath) fail("--output may be specified only once");
      outputPath = path.resolve(value);
    }
    index += 1;
  }

  if (phase !== "screening") fail(`unsupported phase: ${phase ?? "missing"}`);
  if (!outputPath) fail("--output is required");
  return { campaignPath, outputPath, phase, resume };
}

function readCampaign(campaignPath) {
  let source;
  try {
    source = fs.readFileSync(campaignPath, "utf8");
  } catch {
    fail("campaign could not be read");
  }

  let campaign;
  try {
    campaign = JSON.parse(source);
  } catch {
    fail("campaign is not valid JSON");
  }
  if (
    campaign?.schema_version !== 1 ||
    typeof campaign.campaign_id !== "string" ||
    !Array.isArray(campaign.task_families) ||
    !Array.isArray(campaign.candidates?.models) ||
    !Array.isArray(campaign.candidates?.efforts) ||
    !Array.isArray(campaign.candidates?.mechanical_additional_efforts) ||
    !Number.isInteger(
      campaign.sampling?.screening?.distinct_cases_per_task_family,
    )
  ) {
    fail("campaign lacks the required screening contract");
  }

  return {
    campaign,
    sourceSha256: crypto.createHash("sha256").update(source).digest("hex"),
    caseCatalogSha256: hashSibling(campaignPath, "cases.json", "case catalog"),
    fixtureSpecsSha256: hashSibling(
      campaignPath,
      "fixture-specs.json",
      "fixture specs",
    ),
  };
}

function hashSibling(campaignPath, name, label) {
  try {
    return crypto
      .createHash("sha256")
      .update(fs.readFileSync(path.join(path.dirname(campaignPath), name)))
      .digest("hex");
  } catch {
    fail(`${label} could not be read`);
  }
}

function jobId({ taskFamily, caseIndex, model, effort }) {
  return [
    taskFamily,
    `case-${String(caseIndex).padStart(3, "0")}`,
    model,
    effort,
  ].join("/");
}

function buildJobs(campaign) {
  const caseCount = campaign.sampling.screening.distinct_cases_per_task_family;
  const jobs = [];

  for (const taskFamily of campaign.task_families) {
    const efforts =
      taskFamily === "mechanical-assistance"
        ? [
            ...campaign.candidates.efforts,
            ...campaign.candidates.mechanical_additional_efforts,
          ]
        : campaign.candidates.efforts;
    for (let caseIndex = 1; caseIndex <= caseCount; caseIndex += 1) {
      for (const model of campaign.candidates.models) {
        for (const effort of efforts) {
          const identity = { taskFamily, caseIndex, model, effort };
          jobs.push({
            job_id: jobId(identity),
            task_family: taskFamily,
            case_index: caseIndex,
            model,
            effort,
            status: "pending",
            attempt_count: 0,
            result_ref: null,
          });
        }
      }
    }
  }

  return jobs;
}

function buildPlan(
  campaign,
  sourceSha256,
  caseCatalogSha256,
  fixtureSpecsSha256,
  phase,
) {
  return {
    schema_version: 1,
    campaign_id: campaign.campaign_id,
    campaign_source_sha256: sourceSha256,
    case_catalog_sha256: caseCatalogSha256,
    fixture_specs_sha256: fixtureSpecsSha256,
    phase,
    jobs: buildJobs(campaign),
  };
}

function mergeResume(planned, outputPath) {
  let previous;
  try {
    previous = JSON.parse(fs.readFileSync(outputPath, "utf8"));
  } catch {
    fail("resume plan could not be read as JSON");
  }

  if (
    previous?.schema_version !== planned.schema_version ||
    previous.campaign_id !== planned.campaign_id ||
    previous.campaign_source_sha256 !== planned.campaign_source_sha256 ||
    previous.case_catalog_sha256 !== planned.case_catalog_sha256 ||
    previous.fixture_specs_sha256 !== planned.fixture_specs_sha256 ||
    previous.phase !== planned.phase
  ) {
    fail("resume campaign identity does not match the current campaign");
  }
  if (!Array.isArray(previous.jobs)) fail("resume jobs must be an array");

  const previousById = new Map();
  for (const job of previous.jobs) {
    if (
      !job ||
      typeof job.job_id !== "string" ||
      previousById.has(job.job_id)
    ) {
      fail("resume jobs contain a missing or duplicate job identity");
    }
    previousById.set(job.job_id, job);
  }

  const plannedIds = new Set(planned.jobs.map((job) => job.job_id));
  if (
    previousById.size !== plannedIds.size ||
    [...previousById.keys()].some((id) => !plannedIds.has(id))
  ) {
    fail("resume job identities do not match the fixed campaign plan");
  }

  const identityFields = [
    "job_id",
    "task_family",
    "case_index",
    "model",
    "effort",
  ];
  for (const plannedJob of planned.jobs) {
    const previousJob = previousById.get(plannedJob.job_id);
    if (
      identityFields.some((field) => previousJob[field] !== plannedJob[field])
    ) {
      fail(`resume job identity fields do not match: ${plannedJob.job_id}`);
    }
  }

  return {
    ...planned,
    jobs: planned.jobs.map((job) => previousById.get(job.job_id)),
  };
}

function writeAtomically(outputPath, document) {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const temporaryPath = `${outputPath}.tmp-${process.pid}`;
  try {
    fs.writeFileSync(temporaryPath, `${JSON.stringify(document, null, 2)}\n`, {
      flag: "wx",
      mode: 0o600,
    });
    fs.renameSync(temporaryPath, outputPath);
  } finally {
    try {
      fs.unlinkSync(temporaryPath);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
}

function main() {
  try {
    const options = parseArguments(process.argv.slice(2));
    const { campaign, sourceSha256, caseCatalogSha256, fixtureSpecsSha256 } =
      readCampaign(options.campaignPath);
    let plan = buildPlan(
      campaign,
      sourceSha256,
      caseCatalogSha256,
      fixtureSpecsSha256,
      options.phase,
    );
    if (options.resume) plan = mergeResume(plan, options.outputPath);
    writeAtomically(options.outputPath, plan);
    console.log(
      `model-routing ${options.phase} plan ready: ${plan.jobs.length} jobs`,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 2;
  }
}

main();
