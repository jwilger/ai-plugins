#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

function fail(message) {
  throw new Error(`model-routing runner: ${message}`);
}

function parseArguments(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || !value || value.startsWith("--")) {
      fail(
        "usage: --plan PLAN --job-id ID --cases CASES --results DIR --provider MODULE",
      );
    }
    const name = key.slice(2).replaceAll("-", "_");
    if (options[name]) fail(`${key} may be specified only once`);
    options[name] = value;
  }
  for (const name of ["plan", "job_id", "cases", "results", "provider"]) {
    if (!options[name]) fail(`--${name.replaceAll("_", "-")} is required`);
  }
  return {
    planPath: path.resolve(options.plan),
    jobId: options.job_id,
    casesPath: path.resolve(options.cases),
    resultsRoot: path.resolve(options.results),
    providerPath: path.resolve(options.provider),
  };
}

function readJson(filePath, label) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    fail(`${label} could not be read as JSON`);
  }
}

function resolveJob(plan, jobId) {
  if (!Array.isArray(plan?.jobs)) fail("plan jobs must be an array");
  const matches = plan.jobs.filter((job) => job?.job_id === jobId);
  if (matches.length !== 1) fail("job id must resolve exactly once");
  const job = matches[0];
  if (job.status !== "pending") fail(`job is not pending: ${job.status}`);
  return job;
}

function resolveCase(catalog, job) {
  if (catalog?.schema_version !== 1 || !Array.isArray(catalog.cases)) {
    fail("case catalog has an unsupported schema");
  }
  const matches = catalog.cases.filter(
    (item) =>
      item?.task_family === job.task_family &&
      item?.case_index === job.case_index,
  );
  if (matches.length !== 1) fail("fixed case must resolve exactly once");
  const fixedCase = matches[0];
  if (
    typeof fixedCase.case_id !== "string" ||
    typeof fixedCase.prompt !== "string" ||
    typeof fixedCase.verifier?.kind !== "string"
  ) {
    fail("fixed case is incomplete");
  }
  return fixedCase;
}

const executionSurfaceByFamily = {
  "mechanical-assistance": "workspace-write",
  "research-and-discovery": "workspace-read",
  implementation: "workspace-write",
  review: "text-only",
  debugging: "workspace-write",
  "architecture-and-advice": "text-only",
};

function writeAtomically(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const temporaryPath = `${filePath}.tmp-${process.pid}`;
  try {
    fs.writeFileSync(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, {
      flag: "wx",
      mode: 0o600,
    });
    fs.renameSync(temporaryPath, filePath);
  } finally {
    try {
      fs.unlinkSync(temporaryPath);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
}

function validateProviderResult(result) {
  const tokens = result?.usage;
  if (
    result?.status !== "success" ||
    typeof result.output !== "string" ||
    !Number.isInteger(result.elapsed_ms) ||
    result.elapsed_ms < 0 ||
    !Number.isInteger(result.attempts) ||
    result.attempts < 1 ||
    !tokens ||
    ![
      tokens.input_tokens,
      tokens.cached_input_tokens,
      tokens.output_tokens,
    ].every((value) => Number.isInteger(value) && value >= 0)
  ) {
    fail("provider returned an invalid successful result");
  }
}

function recoverPersistedResult(resultPath, expected) {
  if (!fs.existsSync(resultPath)) return null;
  const result = readJson(resultPath, "persisted result");
  if (
    result?.schema_version !== 1 ||
    result.campaign_id !== expected.campaignId ||
    result.phase !== expected.phase ||
    JSON.stringify(result.request) !== JSON.stringify(expected.request) ||
    result.outcome?.status !== "success" ||
    !Number.isInteger(result.attempts) ||
    result.attempts < 1
  ) {
    fail("persisted result does not match the pending job");
  }
  return result;
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  const plan = readJson(options.planPath, "plan");
  const job = resolveJob(plan, options.jobId);
  const fixedCase = resolveCase(
    readJson(options.casesPath, "case catalog"),
    job,
  );
  const request = {
    job_id: job.job_id,
    case_id: fixedCase.case_id,
    prompt: fixedCase.prompt,
    model: job.model,
    effort: job.effort,
    execution_surface: executionSurfaceByFamily[job.task_family],
    fixture: fixedCase.verifier.fixture,
  };
  if (!request.execution_surface) fail("task family has no execution surface");
  const resultName = `${crypto.createHash("sha256").update(job.job_id).digest("hex")}.json`;
  const resultPath = path.join(options.resultsRoot, resultName);
  const persisted = recoverPersistedResult(resultPath, {
    campaignId: plan.campaign_id,
    phase: plan.phase,
    request,
  });
  if (persisted) {
    job.status = "success";
    job.attempt_count += persisted.attempts;
    job.result_ref = resultName;
    writeAtomically(options.planPath, plan);
    console.log(`model-routing job recovered: ${job.job_id}`);
    return;
  }

  const providerModule = await import(pathToFileURL(options.providerPath));
  if (typeof providerModule.default !== "function") {
    fail("provider module must export a default function");
  }
  const providerResult = await providerModule.default(request);
  validateProviderResult(providerResult);

  const result = {
    schema_version: 1,
    campaign_id: plan.campaign_id,
    phase: plan.phase,
    request,
    case: fixedCase,
    outcome: { status: providerResult.status, output: providerResult.output },
    cost: { subject: providerResult.usage, judges: null },
    elapsed_ms: providerResult.elapsed_ms,
    attempts: providerResult.attempts,
  };
  writeAtomically(resultPath, result);
  job.status = "success";
  job.attempt_count += providerResult.attempts;
  job.result_ref = resultName;
  writeAtomically(options.planPath, plan);
  console.log(`model-routing job complete: ${job.job_id}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
});
