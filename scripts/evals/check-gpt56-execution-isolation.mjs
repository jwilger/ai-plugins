#!/usr/bin/env node

import fs from "node:fs";
import {
  allowedNormalizedItemTypes,
  allowedRawResponseItemTypes,
  toolNotificationDescription,
  turnLifecycleRejection,
} from "../../evals/benchmarks/gpt-5.6-model-family/trace-policy.mjs";

const resultsPath = process.argv[2];

if (!resultsPath) {
  console.error("usage: check-gpt56-execution-isolation.mjs <results.json>");
  process.exit(2);
}

const rawArtifact = JSON.parse(fs.readFileSync(resultsPath, "utf8"));
const results = rawArtifact.results?.results || rawArtifact.results || [];

if (!Array.isArray(results) || results.length === 0) {
  console.error(`no GPT-5.6 execution results found: ${resultsPath}`);
  process.exit(1);
}

const prohibitedToolNames = new Set([
  "close_agent",
  "followup_task",
  "interrupt_agent",
  "resume_agent",
  "send_message",
  "spawn_agent",
  "wait_agent",
]);

function providerLabel(result) {
  return String(
    result.provider?.label ||
      result.provider?.id ||
      result.provider ||
      "unknown",
  );
}

function caseLabel(result) {
  const vars = result.testCase?.vars || result.vars || {};
  const caseId = vars.case_id || result.description || "unknown-case";
  const sample = vars.sample_index ? ` sample ${vars.sample_index}` : "";
  return `${providerLabel(result)} :: ${caseId}${sample}`;
}

function parseCodexRaw(result) {
  const value = result.response?.raw;
  if (value && typeof value === "object") {
    return value;
  }
  if (typeof value !== "string" || value.length === 0) {
    throw new Error("missing structured Codex response.raw trace");
  }
  try {
    return JSON.parse(value);
  } catch (error) {
    throw new Error(
      `invalid structured Codex response.raw trace: ${error.message}`,
    );
  }
}

function structuredNames(value) {
  return [
    value.name,
    value.tool,
    value.tool_name,
    value.toolName,
    value.function?.name,
  ]
    .filter((candidate) => typeof candidate === "string")
    .map((candidate) => candidate.toLowerCase());
}

function containsCollaborationActivity(items) {
  const stack = Array.isArray(items) ? [...items] : [];

  while (stack.length > 0) {
    const value = stack.pop();
    if (!value || typeof value !== "object") {
      continue;
    }

    const type = typeof value.type === "string" ? value.type.toLowerCase() : "";
    if (type.includes("collab") || type.includes("subagent")) {
      return true;
    }
    if (structuredNames(value).some((name) => prohibitedToolNames.has(name))) {
      return true;
    }

    for (const nested of Object.values(value)) {
      if (nested && typeof nested === "object") {
        if (Array.isArray(nested)) {
          stack.push(...nested);
        } else {
          stack.push(nested);
        }
      }
    }
  }

  return false;
}

const failures = [];

for (const result of results) {
  try {
    const trace = parseCodexRaw(result);
    if (!Array.isArray(trace.items)) {
      failures.push(
        `${caseLabel(result)}: structured Codex trace has no items array`,
      );
    } else if (containsCollaborationActivity(trace.items)) {
      failures.push(
        `${caseLabel(result)}: collaboration or subagent activity was recorded`,
      );
    } else {
      const rejectedItem = trace.items.find(
        (item) => !allowedNormalizedItemTypes.has(item?.type),
      );
      if (rejectedItem) {
        failures.push(
          `${caseLabel(result)}: disallowed normalized item ${rejectedItem?.type ?? "unknown"}`,
        );
        continue;
      }

      if (!Array.isArray(trace.notifications)) {
        failures.push(`${caseLabel(result)}: no verifiable raw response items`);
        continue;
      }
      if (
        trace.notifications.some(
          (notification) => notification?.method === "turn/plan/updated",
        )
      ) {
        failures.push(
          `${caseLabel(result)}: plan-update activity was recorded`,
        );
        continue;
      }
      const rejectedNotification = toolNotificationDescription(
        trace.notifications,
      );
      if (rejectedNotification) {
        failures.push(
          `${caseLabel(result)}: tool notification ${rejectedNotification} was recorded`,
        );
        continue;
      }

      const rawResponseItems = trace.notifications
        .filter(
          (notification) =>
            notification?.method === "rawResponseItem/completed",
        )
        .map((notification) => notification?.params?.item);
      if (
        rawResponseItems.length === 0 ||
        rawResponseItems.some(
          (item) => !item || typeof item.type !== "string" || !item.type,
        )
      ) {
        failures.push(`${caseLabel(result)}: no verifiable raw response items`);
        continue;
      }

      const rejectedRawItem = rawResponseItems.find(
        (item) => !allowedRawResponseItemTypes.has(item.type),
      );
      if (rejectedRawItem) {
        failures.push(
          `${caseLabel(result)}: disallowed raw response item ${rejectedRawItem.type}`,
        );
      }
      if (!Array.isArray(trace.serverRequests)) {
        failures.push(
          `${caseLabel(result)}: no verifiable server request trace`,
        );
      } else if (trace.serverRequests.length > 0) {
        failures.push(
          `${caseLabel(result)}: server request activity was recorded`,
        );
      }

      const lifecycleRejection = turnLifecycleRejection(trace.notifications);
      if (lifecycleRejection) {
        failures.push(`${caseLabel(result)}: ${lifecycleRejection}`);
      }
    }
  } catch (error) {
    failures.push(`${caseLabel(result)}: ${error.message}`);
  }
}

if (failures.length > 0) {
  console.error("GPT-5.6 execution isolation failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.error(
  `verified ${results.length} direct GPT-5.6 execution result${results.length === 1 ? "" : "s"} with complete message-only traces`,
);
