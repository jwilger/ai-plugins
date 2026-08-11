#!/usr/bin/env node

import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

const [command, ...args] = process.argv.slice(2);
const projectRoot = process.env.FINAL_REVIEW_TEST_PROJECT_ROOT;
const baselineCommit = process.env.FINAL_REVIEW_TEST_BASELINE_COMMIT;
const sessionId = process.env.FINAL_REVIEW_TEST_SESSION_ID;
if (!command || !projectRoot || !baselineCommit || !sessionId) {
  throw new Error("advisory persistence fixture arguments are required");
}

const activeServers = new Set();

function startServer() {
  const child = spawn(command, args, {
    env: process.env,
    stdio: ["pipe", "pipe", "inherit"],
  });
  const lines = createInterface({ input: child.stdout });
  const pending = [];
  let closeCode;
  const rejectPending = (error) => {
    while (pending.length > 0) {
      const next = pending.shift();
      clearTimeout(next.timeout);
      next.reject(error);
    }
  };
  lines.on("line", (line) => {
    const next = pending.shift();
    if (!next) return;
    clearTimeout(next.timeout);
    try {
      next.resolve(JSON.parse(line));
    } catch (error) {
      next.reject(error);
    }
  });
  child.on("error", rejectPending);
  const closed = new Promise((resolve) => {
    child.once("close", (code) => {
      closeCode = code;
      rejectPending(
        new Error(`MCP closed before responding with status ${code}`),
      );
      resolve(code);
    });
  });
  const server = {
    child,
    closed,
    closeCode: () => closeCode,
    request(payload) {
      return new Promise((resolve, reject) => {
        const timeout = setTimeout(
          () => reject(new Error("MCP request timed out after 15 seconds")),
          15_000,
        );
        pending.push({ resolve, reject, timeout });
        child.stdin.write(
          `${JSON.stringify(payload)}\n`,
          (error) => error && reject(error),
        );
      });
    },
  };
  activeServers.add(server);
  return server;
}

async function initialize(server, id) {
  const response = await server.request({
    jsonrpc: "2.0",
    id,
    method: "initialize",
    params: {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "signed-advisory-test", version: "0.0.0" },
    },
  });
  requireToolResult(response, "initialize", false);
}

async function stop(server) {
  if (!activeServers.has(server)) return;
  if (server.closeCode() === undefined) server.child.stdin.end();
  const code = await Promise.race([
    server.closed,
    new Promise((_, reject) =>
      setTimeout(
        () => reject(new Error("MCP close timed out after 5 seconds")),
        5_000,
      ),
    ),
  ]);
  activeServers.delete(server);
  if (code !== 0) throw new Error(`MCP exited with status ${code}`);
}

function requireToolResult(response, operation, parseText = true) {
  if (!response.result || response.result.isError === true) {
    throw new Error(`${operation} failed: ${JSON.stringify(response)}`);
  }
  if (!parseText) return response.result;
  if (response.result.content?.[0]?.type !== "text") {
    throw new Error(`${operation} returned no text result`);
  }
  return JSON.parse(response.result.content[0].text);
}

const common = {
  session_id: sessionId,
  baseline_commit: baselineCommit,
  scope: "uncommitted",
  project_root: projectRoot,
  changed_files: ["src/new.rs"],
  diff_hash: "signed-advisory-fixture",
  shared_test_evidence: {
    id: "signed-advisory-evidence",
    diff_hash: "signed-advisory-fixture",
    status: "passed",
    summary: "The signed packaged advisory persistence fixture passed.",
    commands: ["fixture:signed-advisory-persistence"],
  },
};

function cleanLensResults(state) {
  return state.lenses.map((lens) => ({
    lens,
    subagent_key: `${state.session_id}:${state.iteration_index}:${lens}`,
    status: "clean",
    shared_test_evidence_id: state.shared_test_evidence.id,
    additional_broad_test_run: false,
    caller_attestation: {
      model_role: [
        "architecture-maintainability",
        "security-safety",
        "safety-human-harm",
      ].includes(lens)
        ? state.model_roles.verifier
        : state.model_roles.lens_review,
      fresh_context: true,
      closed_after_result: true,
    },
  }));
}

async function run() {
  const first = startServer();
  await initialize(first, 1);
  const assessed = requireToolResult(
    await first.request({
      jsonrpc: "2.0",
      id: 2,
      method: "tools/call",
      params: { name: "final_review.assess_risk", arguments: common },
    }),
    "assess",
  );
  const assignment = assessed.assignments[0];
  const dimensions = assignment.review_dimensions.map((lens, index) => ({
    lens,
    risk: index === 0 ? "low" : "none",
    evidence:
      index === 0
        ? "The shared Git-backed append path is the behavior under test."
        : "No additional failure path in this bounded fixture.",
    plausible_failure:
      index === 0 ? "The review session is not persisted." : "none",
    material_impact:
      index === 0 ? "Review orchestration cannot resume." : "none",
    uncertain: false,
  }));
  const riskAssessment = {
    assignment_id: assignment.assignment_id,
    subagent_key: assignment.subagent_key,
    shared_test_evidence_id: assignment.shared_test_evidence.id,
    overall_risk: "low",
    dimensions,
    exceptional_triggers: [],
    split_required: false,
    plan_assumptions: [],
    findings: [],
    caller_attestation: {
      model_role: assignment.model_role,
      fresh_context: true,
      closed_after_result: true,
    },
  };
  const plan = requireToolResult(
    await first.request({
      jsonrpc: "2.0",
      id: 3,
      method: "tools/call",
      params: {
        name: "final_review.plan",
        arguments: { ...common, risk_assessment: riskAssessment },
      },
    }),
    "plan",
  );
  await stop(first);

  const second = startServer();
  await initialize(second, 4);
  const resume = requireToolResult(
    await second.request({
      jsonrpc: "2.0",
      id: 5,
      method: "tools/call",
      params: {
        name: "final_review.resume_latest",
        arguments: { session_id: sessionId, project_root: projectRoot },
      },
    }),
    "resume",
  );
  if (resume.state_ref.state_fingerprint !== plan.state_ref.state_fingerprint) {
    throw new Error("resumed state does not match planned state");
  }

  let state = plan.state;
  let stateRef = resume.state_ref;
  let transition;
  for (let iteration = 0; iteration < 4; iteration += 1) {
    transition = requireToolResult(
      await second.request({
        jsonrpc: "2.0",
        id: 6 + iteration,
        method: "tools/call",
        params: {
          name: "final_review.advance",
          arguments: {
            state_ref: stateRef,
            lens_results: cleanLensResults(state),
            current_diff_hash: common.diff_hash,
          },
        },
      }),
      "advance",
    );
    state = transition.state;
    stateRef = transition.state_ref;
    if (transition.complete) break;
  }
  if (!transition?.complete)
    throw new Error("advisory review did not complete");
  const clean = requireToolResult(
    await second.request({
      jsonrpc: "2.0",
      id: 10,
      method: "tools/call",
      params: {
        name: "final_review.clean_status",
        arguments: { state_ref: stateRef },
      },
    }),
    "clean_status",
  );
  if (!clean.complete) throw new Error("completed advisory state is not clean");
  await stop(second);
  process.stdout.write(`${JSON.stringify({ state_ref: stateRef })}\n`);
}

try {
  await run();
} finally {
  const closing = [];
  for (const server of activeServers) {
    if (server.closeCode() === undefined) server.child.kill("SIGTERM");
    closing.push(server.closed);
  }
  await Promise.allSettled(closing);
}
