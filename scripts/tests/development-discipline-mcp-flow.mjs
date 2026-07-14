#!/usr/bin/env node

import { execFileSync, spawn } from "node:child_process";
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
const ticketBaselineCommit = execFileSync(
  "git",
  ["-C", projectRoot, "rev-parse", "--verify", "HEAD^{commit}"],
  { encoding: "utf8" },
).trim();

const child = spawn(command, args, {
  env: {
    ...process.env,
    XDG_STATE_HOME: `${projectRoot}/.development-discipline-state`,
  },
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
  return reviewState.lenses.map((lens) => {
    const result = {
      lens,
      subagent_key: `${reviewState.session_id}:${reviewState.iteration_index}:${lens}`,
      status: "clean",
      caller_attestation: {
        model_role: reviewState.model_roles.lens_review,
        fresh_context: true,
        closed_after_result: true,
      },
    };
    if (reviewState.shared_test_evidence) {
      result.shared_test_evidence_id = reviewState.shared_test_evidence.id;
      result.additional_broad_test_run = false;
    }
    return result;
  });
}

const findingLensResults = cleanLensResults(state);
findingLensResults[0] = {
  ...findingLensResults[0],
  status: "findings",
  findings: [
    {
      id: "launcher-real",
      severity: "CRITICAL",
      causality: "caused",
      causality_evidence: "The fixture attributes the candidate to src/new.rs.",
      likelihood: "possible",
      security_impact: "none",
      safety_impact: "none",
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
      causality: "pre-existing",
      causality_evidence: "The fixture places the candidate in unchanged code.",
      likelihood: "observed",
      security_impact: "none",
      safety_impact: "none",
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
          model_role: verifierAssignment.model_role,
          fresh_context: true,
          closed_after_result: true,
        },
        verdicts: [
          {
            finding_id: "launcher-real",
            lens: "correctness-behavior",
            verdict: "rejected",
            severity: "CRITICAL",
            causality: "caused",
            causality_evidence:
              "The verifier established that the reported scenario is unreachable.",
            security_impact: "none",
            safety_impact: "none",
            rationale:
              "The launcher fixture intentionally exercises rejection.",
          },
        ],
      },
    },
  },
});
let currentState = JSON.parse(verifiedResponse.result.content[0].text).state;
for (let index = 0; index < 2; index += 1) {
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
    causality: "pre-existing",
    causality_evidence: "The fixture places the candidate in unchanged code.",
    likelihood: "observed",
    path: "src/old.rs",
    message: "alice@example.test exploit payload",
    scenario: "private data",
    security_impact: "major",
    safety_impact: "none",
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

const ticketRiskArguments = {
  session_id: "bats-verifier-ticket-evidence",
  base: "HEAD",
  baseline_commit: ticketBaselineCommit,
  scope: "uncommitted",
  project_root: projectRoot,
  changed_files: ["src/new.rs"],
  diff_hash: "ticket-evidence",
  user_request: "Review the changed local tooling behavior.",
  acceptance_criteria: ["Disposition confirmed findings without deadlock."],
  unrelated_finding_policy: { default: "report" },
  shared_test_evidence: {
    id: "tests-ticket-evidence",
    diff_hash: "ticket-evidence",
    status: "passed",
    summary: "Fast fixture tests passed for this diff.",
    commands: ["fixture:fast-tests"],
    artifact_reference: "fixture://fast-tests/ticket-evidence",
  },
};
const ticketScoutResponse = await request(
  {
    jsonrpc: "2.0",
    id: 17,
    method: "tools/call",
    params: {
      name: "final_review.assess_risk",
      arguments: ticketRiskArguments,
    },
  },
  false,
);
const ticketScout = JSON.parse(ticketScoutResponse.result.content[0].text)
  .assignments[0];
const ticketDimensions = ticketScout.review_dimensions.map((lens) => {
  const selected = lens === "correctness-behavior";
  return {
    lens,
    risk: selected ? "high" : "none",
    evidence: selected
      ? "The changed disposition transition can deadlock the coordinator."
      : "No concrete failure path for this dimension.",
    plausible_failure: selected
      ? "A confirmed nonblocking finding cannot be documented."
      : "none",
    material_impact: selected
      ? "Review completion becomes impossible."
      : "none",
    uncertain: false,
  };
});
const ticketPlanResponse = await request(
  {
    jsonrpc: "2.0",
    id: 18,
    method: "tools/call",
    params: {
      name: "final_review.plan",
      arguments: {
        ...ticketRiskArguments,
        risk_assessment: {
          assignment_id: ticketScout.assignment_id,
          subagent_key: ticketScout.subagent_key,
          shared_test_evidence_id: ticketScout.shared_test_evidence.id,
          overall_risk: "high",
          dimensions: ticketDimensions,
          exceptional_triggers: [],
          split_required: false,
          plan_assumptions: [],
          findings: [],
          caller_attestation: {
            model_role: ticketScout.model_role,
            fresh_context: true,
            closed_after_result: true,
          },
        },
      },
    },
  },
  false,
);
const ticketState = JSON.parse(ticketPlanResponse.result.content[0].text).state;
const ticketLensResults = cleanLensResults(ticketState);
ticketLensResults[0] = {
  ...ticketLensResults[0],
  status: "findings",
  findings: [
    {
      id: "material-auth-regression",
      severity: "MAJOR",
      causality: "caused",
      causality_evidence:
        "The changed branch appears to disclose protected diagnostics.",
      likelihood: "possible",
      security_impact: "major",
      safety_impact: "none",
      path: "src/new.rs",
      message: "The changed branch may disclose protected diagnostics.",
      relevance: {
        category: "diff_changed_file",
        explanation: "The branch is changed by this diff.",
      },
    },
  ],
};
const ticketPendingResponse = await request(
  {
    jsonrpc: "2.0",
    id: 19,
    method: "tools/call",
    params: {
      name: "final_review.advance",
      arguments: {
        state: ticketState,
        lens_results: ticketLensResults,
        current_diff_hash: "ticket-evidence",
      },
    },
  },
  false,
);
if (!ticketPendingResponse.result) {
  throw new Error(
    `verifier ticket setup failed: ${JSON.stringify(ticketPendingResponse)}`,
  );
}
const ticketVerifier = JSON.parse(
  ticketPendingResponse.result.content[0].text,
).verifier_assignment;
const ticketAdvancedResponse = await request(
  {
    jsonrpc: "2.0",
    id: 20,
    method: "tools/call",
    params: {
      name: "final_review.advance",
      arguments: {
        state: ticketState,
        lens_results: ticketLensResults,
        current_diff_hash: "ticket-evidence",
        unrelated_follow_ups: [
          {
            finding_id: "material-auth-regression",
            lens: "correctness-behavior",
            ticket_reference: "BACKLOG-SEC-1",
          },
        ],
        verifier_result: {
          subagent_key: ticketVerifier.subagent_key,
          assignment_id: ticketVerifier.assignment_id,
          model_role: ticketVerifier.model_role,
          status: "verified",
          verdicts: [
            {
              finding_id: "material-auth-regression",
              lens: "correctness-behavior",
              verdict: "confirmed",
              severity: "MINOR",
              causality: "caused",
              causality_evidence:
                "The diff causes only a minor diagnostic disclosure.",
              security_impact: "minor",
              safety_impact: "none",
              rationale: "The confirmed impact belongs in the backlog.",
            },
          ],
          caller_attestation: {
            model_role: ticketVerifier.model_role,
            fresh_context: true,
            closed_after_result: true,
          },
        },
      },
    },
  },
  false,
);
if (!ticketAdvancedResponse.result) {
  throw new Error(
    `verifier ticket resubmission failed: ${JSON.stringify(ticketAdvancedResponse)}`,
  );
}
const ticketAdvanced = JSON.parse(
  ticketAdvancedResponse.result.content[0].text,
);
if (
  ticketAdvanced.transition_status !== "advanced" ||
  ticketAdvanced.filtered.routed[0]?.disposition !== "ticket" ||
  ticketAdvanced.state.deferred_findings[0]?.ticket_reference !==
    "BACKLOG-SEC-1"
) {
  throw new Error("verifier ticket evidence did not advance the MCP session");
}

child.stdin.end();
const exitCode = await new Promise((resolve) => child.on("close", resolve));
if (exitCode !== 0) {
  throw new Error(`development-discipline MCP exited with status ${exitCode}`);
}
