---
title: Make the marketplace canary accept the capability descriptions it asks for
blocked_by: []
blocks: []
tags: [evals, canary, prompt-contract, codex]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Align the full-marketplace canary checker with its prompt so an accurate plugin capability can satisfy the check without requiring an undocumented literal skill name.

## Context / Why

The canary prompt asks each harness to name every marketplace plugin and provide at least one representative skill or capability. On two consecutive live runs, Codex named all eight plugins and accurately described eval-case-reporter as creating scrubbed, approval-gated eval reports. The checker still failed because it accepts only the literal skill directory name submit-eval-case. This creates false loading failures even when the response proves the plugin is available.

## Acceptance criteria

- [ ] The canary prompt and checker use the same definition of an acceptable representative skill or capability.

## Subtasks

## Notes / Log

- 2026-07-22: C2BU reproduced the false negative twice on 2026-07-22. Codex named all eight marketplace plugins and accurately described `development-discipline` (preflight/TDD/debugging/final review/verification/delivery) and `eval-case-reporter` (scrubbed approval-gated eval reports), yet the checker reported both as missing representative skills. Artifacts: `evals/out/c2bu-canary-retry/results.json` and `evals/out/c2bu-canary-retry-2/results.json`. Claude passed both runs. This is checker/prompt contract evidence, not a plugin-loading failure.
- 2026-07-22: WTK6 reproduced the same false negative on exact artifact evals/out/wtk6-canary/results.json: Codex listed all eight marketplace plugins and accurately described worktrees as safe parallel-worktree setup and eval-case-reporter as scrubbed eval-case reporting with approval, but the checker rejected both because literal representative skill names were absent.
- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
