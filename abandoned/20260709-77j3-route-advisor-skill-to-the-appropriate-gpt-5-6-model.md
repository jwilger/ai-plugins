---
title: Route advisor skill to the appropriate GPT-5.6 model
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Update the advisor skill's Codex model routing for the GPT-5.6 Sol, Terra, and Luna family, selecting the model that best balances advisory quality, latency, and cost while preserving Claude support and configurability.

## Context / Why

## Acceptance criteria

- [ ] The advisor skill uses an explicitly justified GPT-5.6 Codex model appropriate for high-value architecture and tradeoff advice.
- [ ] The routing remains configurable and does not impose a Codex model on Claude or unsupported harnesses.
- [ ] Behavior tests and eval cases cover the selected routing and fallback behavior.

## Subtasks

## Notes / Log

- 2026-07-12: Backlog grooming: superseded by 20260711-wtk6 after the user explicitly selected gpt-5.6-sol with high reasoning for the intentionally Codex-only advisor plugin. Do not carry the older unresolved model-selection or Claude-support scope forward.
- 2026-07-12: Backlog grooming follow-up: the older generic configurability and fallback scope is also intentionally superseded. Canonical 20260711-wtk6 pins the user's exact route, adds no custom override layer, and requires visible failure rather than silent downgrade.
