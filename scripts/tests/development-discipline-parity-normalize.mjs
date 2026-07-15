#!/usr/bin/env node

import { readFileSync } from "node:fs";

const [inputPath] = process.argv.slice(2);
if (!inputPath) {
  throw new Error(
    "usage: development-discipline-parity-normalize.mjs <jsonl-output>",
  );
}

const contractIds = new Map();
const transitionIds = new Map();

function normalizedContractId(contractId) {
  if (!contractIds.has(contractId)) {
    contractIds.set(contractId, `<review-contract-${contractIds.size + 1}>`);
  }
  return contractIds.get(contractId);
}

function normalizedTransitionId(transitionId) {
  if (!transitionIds.has(transitionId)) {
    transitionIds.set(
      transitionId,
      `<review-transition-${transitionIds.size + 1}>`,
    );
  }
  return transitionIds.get(transitionId);
}

function normalizeVerifiedTransitions(state) {
  const transitions = state.verified_clean_iterations;
  if (!Array.isArray(transitions)) {
    return;
  }
  for (const transition of transitions) {
    if (
      !Number.isSafeInteger(transition?.iteration) ||
      transition.iteration < 0 ||
      typeof transition.transition_id !== "string" ||
      !/^[0-9a-f]{16}$/.test(transition.transition_id)
    ) {
      continue;
    }
    transition.transition_id = normalizedTransitionId(transition.transition_id);
  }
}

function normalizeReviewState(payload) {
  const state = payload?.state;
  if (!state || typeof state !== "object") {
    return false;
  }

  const startedAt = state?.risk_plan?.review_budget?.started_at_epoch_seconds;
  const contractId = state.review_contract_id;
  if (
    !Number.isSafeInteger(startedAt) ||
    startedAt < 0 ||
    typeof contractId !== "string" ||
    !/^[0-9a-f]{16}$/.test(contractId)
  ) {
    return false;
  }

  state.risk_plan.review_budget.started_at_epoch_seconds = 0;
  state.review_contract_id = normalizedContractId(contractId);
  normalizeVerifiedTransitions(state);
  return true;
}

function normalizeResponse(response) {
  const content = response?.result?.content;
  if (!Array.isArray(content)) {
    return response;
  }
  for (const item of content) {
    if (item?.type !== "text" || typeof item.text !== "string") {
      continue;
    }
    try {
      const payload = JSON.parse(item.text);
      if (normalizeReviewState(payload)) {
        item.text = JSON.stringify(payload);
      }
    } catch {
      // Ordinary diagnostic and reviewer-prompt text is not parity state.
    }
  }
  return response;
}

const lines = readFileSync(inputPath, "utf8").split("\n");
if (lines.at(-1) === "") {
  lines.pop();
}
if (lines.some((line) => line.length === 0)) {
  throw new Error("JSONL output contains a blank record");
}

const normalizedLines = lines.map((line) =>
  JSON.stringify(normalizeResponse(JSON.parse(line))),
);

process.stdout.write(`${normalizedLines.join("\n")}\n`);
