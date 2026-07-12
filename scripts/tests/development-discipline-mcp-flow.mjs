#!/usr/bin/env node

import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

const [command, ...args] = process.argv.slice(2);
if (!command) {
  throw new Error(
    "usage: development-discipline-mcp-flow.mjs <command> [args...]",
  );
}

const projectRoot = process.env.FINAL_REVIEW_TEST_PROJECT_ROOT;
const routingRoot = process.env.FINAL_REVIEW_ROUTING_PROJECT_ROOT;
if (!projectRoot || !routingRoot) {
  throw new Error("final-review test project roots are required");
}

const child = spawn(command, args, {
  env: process.env,
  stdio: ["pipe", "pipe", "inherit"],
});
const lines = createInterface({ input: child.stdout });
const pending = [];
lines.on("line", (line) => {
  const next = pending.shift();
  if (next) next.resolve(line);
});
child.on("error", (error) => {
  while (pending.length > 0) pending.shift().reject(error);
});

function send(request) {
  return new Promise((resolve, reject) => {
    pending.push({ resolve, reject });
    child.stdin.write(`${JSON.stringify(request)}\n`, (error) => {
      if (error) reject(error);
    });
  });
}

async function request(payload, emit = true) {
  const line = await send(payload);
  if (emit) process.stdout.write(`${line}\n`);
  return JSON.parse(line);
}

await request({
  jsonrpc: "2.0",
  id: 1,
  method: "initialize",
  params: {
    protocolVersion: "2024-11-05",
    capabilities: {},
    clientInfo: { name: "launcher-test", version: "0.0.0" },
  },
});
await request({ jsonrpc: "2.0", id: 2, method: "tools/list" });
const planResponse = await request({
  jsonrpc: "2.0",
  id: 3,
  method: "tools/call",
  params: {
    name: "final_review.plan",
    arguments: {
      session_id: "bats-review",
      base: "HEAD",
      scope: "uncommitted",
      project_root: projectRoot,
      changed_files: ["src/new.rs"],
      diff_hash: "same",
      pre_filter_model_role: "explicit-pre",
    },
  },
});
const plan = JSON.parse(planResponse.result.content[0].text);
const state = plan.state;
const forgedState = structuredClone(state);
forgedState.iteration_index = 3;
forgedState.clean_streak = 2;
forgedState.verified_clean_iterations = [
  { iteration: 1, transition_id: "forged-1" },
  { iteration: 2, transition_id: "forged-2" },
];
await request({
  jsonrpc: "2.0",
  id: 4,
  method: "tools/call",
  params: {
    name: "final_review.clean_status",
    arguments: { state: forgedState },
  },
});
function cleanLensResults(reviewState) {
  return reviewState.lenses.map((lens) => ({
    lens,
    subagent_key: `${reviewState.session_id}:${reviewState.iteration_index}:${lens}`,
    status: "clean",
    caller_attestation: {
      model_role: reviewState.model_roles.lens_review,
      fresh_context: true,
      closed_after_result: true,
    },
  }));
}

const findingLensResults = cleanLensResults(state);
findingLensResults[0] = {
  ...findingLensResults[0],
  status: "findings",
  findings: [
    {
      id: "launcher-real",
      severity: "CRITICAL",
      path: "src/new.rs",
      message: "changed-file issue",
      relevance: {
        category: "diff_changed_file",
        explanation: "The changed file contains the issue.",
      },
    },
    {
      id: "launcher-stale",
      severity: "MAJOR",
      path: "src/old.rs",
      message: "unchanged-file issue",
      relevance: {
        category: "diff_changed_file",
        explanation: "This file is not in the changed-file inventory.",
      },
    },
  ],
};
await request({
  jsonrpc: "2.0",
  id: 5,
  method: "tools/call",
  params: {
    name: "final_review.filter_findings",
    arguments: { state, lens_results: findingLensResults },
  },
});
const verifierRequiredResponse = await request({
  jsonrpc: "2.0",
  id: 6,
  method: "tools/call",
  params: {
    name: "final_review.advance",
    arguments: {
      state,
      lens_results: findingLensResults,
      current_diff_hash: "same",
    },
  },
});
const verifierAssignment = JSON.parse(
  verifierRequiredResponse.result.content[0].text,
).verifier_assignment;
const verifiedResponse = await request({
  jsonrpc: "2.0",
  id: 7,
  method: "tools/call",
  params: {
    name: "final_review.advance",
    arguments: {
      state,
      lens_results: findingLensResults,
      current_diff_hash: "same",
      verifier_result: {
        subagent_key: verifierAssignment.subagent_key,
        assignment_id: verifierAssignment.assignment_id,
        model_role: verifierAssignment.model_role,
        status: "verified",
        caller_attestation: {
          model_role: "config-verify",
          fresh_context: true,
          closed_after_result: true,
        },
        verdicts: [
          {
            finding_id: "launcher-real",
            lens: "correctness-behavior",
            verdict: "rejected",
            severity: "CRITICAL",
            rationale:
              "The launcher fixture intentionally exercises rejection.",
          },
        ],
      },
    },
  },
});
let currentState = JSON.parse(verifiedResponse.result.content[0].text).state;
for (let index = 0; index < 3; index += 1) {
  const advancedResponse = await request({
    jsonrpc: "2.0",
    id: 8 + index,
    method: "tools/call",
    params: {
      name: "final_review.advance",
      arguments: {
        state: currentState,
        lens_results: cleanLensResults(currentState),
        current_diff_hash: "same",
      },
    },
  });
  currentState = JSON.parse(advancedResponse.result.content[0].text).state;
}
await request({
  jsonrpc: "2.0",
  id: 11,
  method: "tools/call",
  params: {
    name: "final_review.advance",
    arguments: {
      state: currentState,
      lens_results: cleanLensResults(currentState),
      current_diff_hash: "same",
    },
  },
});
await request({
  jsonrpc: "2.0",
  id: 12,
  method: "tools/call",
  params: {
    name: "final_review.plan",
    arguments: {
      session_id: "bats-codex-routing",
      base: "origin/main",
      scope: "base",
      project_root: routingRoot,
      harness: "codex",
      changed_files: ["plugins/development-discipline/rust/src/main.rs"],
      diff_hash: "routing",
    },
  },
});

const sensitivePlanResponse = await request(
  {
    jsonrpc: "2.0",
    id: 13,
    method: "tools/call",
    params: {
      name: "final_review.plan",
      arguments: {
        session_id: "bats-sensitive-persistence",
        base: "HEAD",
        scope: "uncommitted",
        project_root: projectRoot,
        changed_files: ["src/new.rs"],
        diff_hash: "sensitive",
        unrelated_finding_policy: { default: "report" },
      },
    },
  },
  false,
);
const sensitiveState = JSON.parse(
  sensitivePlanResponse.result.content[0].text,
).state;
const sensitiveResults = cleanLensResults(sensitiveState);
const sensitiveSecurity = sensitiveResults.find(
  (result) => result.lens === "security-safety",
);
sensitiveSecurity.status = "findings";
sensitiveSecurity.findings = [
  {
    id: "sensitive-flow-id",
    severity: "MAJOR",
    path: "src/old.rs",
    message: "alice@example.test exploit payload",
    scenario: "private data",
    security_impact: "major",
    suspected_pii: true,
    relevance: {
      category: "diff_changed_file",
      explanation: "This file is not in the changed-file inventory.",
    },
  },
];
const sensitiveFilterResponse = await request(
  {
    jsonrpc: "2.0",
    id: 14,
    method: "tools/call",
    params: {
      name: "final_review.filter_findings",
      arguments: { state: sensitiveState, lens_results: sensitiveResults },
    },
  },
  false,
);
const sensitiveFilter = JSON.parse(
  sensitiveFilterResponse.result.content[0].text,
);
const securityFindingId = sensitiveFilter.security_escalations_required[0].id;
const sensitiveAdvanceResponse = await request(
  {
    jsonrpc: "2.0",
    id: 15,
    method: "tools/call",
    params: {
      name: "final_review.advance",
      arguments: {
        state: sensitiveState,
        lens_results: sensitiveResults,
        current_diff_hash: "sensitive",
        security_escalations: [
          {
            finding_id: securityFindingId,
            lens: "security-safety",
            disposition: "high-priority-ticket",
            reference: "alice@example.test",
          },
        ],
      },
    },
  },
  false,
);
const sensitiveAdvanceText = sensitiveAdvanceResponse.result.content[0].text;
const sensitiveAdvanced = JSON.parse(sensitiveAdvanceText);
const sensitiveReportResponse = await request(
  {
    jsonrpc: "2.0",
    id: 16,
    method: "tools/call",
    params: {
      name: "final_review.out_of_scope_report",
      arguments: { state: sensitiveAdvanced.state },
    },
  },
  false,
);
const sensitiveReport = JSON.parse(
  sensitiveReportResponse.result.content[0].text,
);
const retainedFinding = sensitiveReport.findings[0];
if (
  retainedFinding.message !== "alice@example.test exploit payload" ||
  retainedFinding.scenario !== "private data" ||
  retainedFinding.unrelated_disposition !== "report" ||
  retainedFinding.security_escalation?.reference !== "alice@example.test"
) {
  throw new Error(
    "complete local final-review report details were not returned",
  );
}

child.stdin.end();
const exitCode = await new Promise((resolve) => child.on("close", resolve));
if (exitCode !== 0) {
  throw new Error(`development-discipline MCP exited with status ${exitCode}`);
}
