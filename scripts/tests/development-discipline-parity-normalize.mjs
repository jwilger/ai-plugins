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
const reviewBudgetStartTimesBySession = new Map();

function hasNonLosslessJsonNumber(jsonText) {
  for (let index = 0; index < jsonText.length; index += 1) {
    if (jsonText[index] === '"') {
      index += 1;
      while (index < jsonText.length && jsonText[index] !== '"') {
        if (jsonText[index] === "\\") {
          index += 1;
        }
        index += 1;
      }
      continue;
    }

    if (jsonText[index] !== "-" && !/[0-9]/.test(jsonText[index])) {
      continue;
    }
    const numberToken = jsonText
      .slice(index)
      .match(/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/)?.[0];
    if (!numberToken) {
      continue;
    }
    if (JSON.stringify(Number(numberToken)) !== numberToken) {
      return true;
    }
    index += numberToken.length - 1;
  }
  return false;
}

function normalizedReviewBudgetStartTime(sessionId, startedAt) {
  if (!reviewBudgetStartTimesBySession.has(sessionId)) {
    reviewBudgetStartTimesBySession.set(sessionId, new Map());
  }
  const startTimes = reviewBudgetStartTimesBySession.get(sessionId);
  if (!startTimes.has(startedAt)) {
    startTimes.set(startedAt, startTimes.size + 1);
  }
  return startTimes.get(startedAt);
}

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
      transition.iteration <= 0 ||
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
  const sessionId = state.session_id;
  if (
    !Number.isSafeInteger(startedAt) ||
    startedAt < 0 ||
    typeof contractId !== "string" ||
    !/^[0-9a-f]{16}$/.test(contractId) ||
    typeof sessionId !== "string" ||
    !/^[A-Za-z0-9._:-]{1,128}$/.test(sessionId)
  ) {
    return false;
  }

  state.risk_plan.review_budget.started_at_epoch_seconds =
    normalizedReviewBudgetStartTime(sessionId, startedAt);
  state.review_contract_id = normalizedContractId(contractId);
  normalizeVerifiedTransitions(state);
  return true;
}

function normalizeResponse(response) {
  if (typeof response?.error?.message === "string") {
    const normalizedMessage = response.error.message.replaceAll(
      /((?:expected|received)_(?:state|assignment)_fingerprint=)[0-9a-f]{16}/g,
      "$1<opaque-fingerprint>",
    );
    if (normalizedMessage !== response.error.message) {
      response.error.message = normalizedMessage;
      return true;
    }
  }
  const content = response?.result?.content;
  if (!Array.isArray(content)) {
    return false;
  }
  let normalized = false;
  for (const item of content) {
    if (
      item?.type !== "text" ||
      typeof item.text !== "string" ||
      hasNonLosslessJsonNumber(item.text)
    ) {
      continue;
    }
    try {
      const payload = JSON.parse(item.text);
      if (normalizeReviewState(payload)) {
        item.text = JSON.stringify(payload);
        normalized = true;
      }
    } catch {
      // Ordinary diagnostic and reviewer-prompt text is not parity state.
    }
  }
  return normalized;
}

const lines = readFileSync(inputPath, "utf8").split("\n");
if (lines.at(-1) === "") {
  lines.pop();
}
const normalizedLines = lines.map((line, index) => {
  const recordNumber = index + 1;
  if (line.length === 0) {
    throw new Error(
      `JSONL input ${JSON.stringify(inputPath)} record=${recordNumber}: blank record`,
    );
  }

  const preserveLine = hasNonLosslessJsonNumber(line);
  let response;
  try {
    response = JSON.parse(line);
  } catch {
    throw new Error(
      `JSONL input ${JSON.stringify(inputPath)} record=${recordNumber}: invalid JSON`,
    );
  }

  if (preserveLine) {
    return line;
  }

  return normalizeResponse(response) ? JSON.stringify(response) : line;
});

process.stdout.write(`${normalizedLines.join("\n")}\n`);
