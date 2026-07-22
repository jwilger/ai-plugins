---
title: Prove evaluation interruption is handled before process startup finishes
blocked_by: []
blocks: []
tags: [evals, signals, tests, race-condition, minor, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen the launch-timing test so it definitively proves an interrupt is handled before the evaluation process identifier is published. The test must not accidentally pass through the normal post-start path.

## Context / Why

Implementation notes:\n\nA final-review MINOR found that the current BASH_ENV DEBUG-hook fixture pauses immediately before eval_pid="$!", sends SIGINT, and then creates capture.release without waiting for proof that the trap ran. Because signal delivery is asynchronous, the assignment may execute before trap dispatch, so the test can pass even if the eval_launching deferred-signal branch regresses. Add a deterministic trap-executed handshake while keeping the production runner free of test-only hooks.

## Acceptance criteria

- [ ] The regression holds PID publication until it has observed an explicit marker proving the runner's INT handler executed while eval_pid was still empty and the launch phase was active.
- [ ] The test fails when deferred launch-phase signal handling is removed or bypassed, and passes with the intended eval_launching behavior.
- [ ] The fixture remains deterministic and bounded without adding production-only test hooks to scripts/evals/run.sh.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
