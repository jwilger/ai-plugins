---
title: Report whether an interrupted evaluation was still setting up or already running
blocked_by: []
blocks: []
tags: [evals, signals, diagnostics, operability, minor, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Use accurate messages when a user stops an evaluation. An interruption during dependency or configuration setup should not claim that the provider evaluation had already started, while interruptions after launch should retain the existing run-specific wording.

## Context / Why

Implementation notes:\n\nFinal review found a MINOR operability issue in the new pre-Promptfoo signal path. SIGINT during ensure-node-deps.sh, generated configuration, or Codex-home preparation currently routes through finish_eval_interruption and persists `promptfoo eval was interrupted before completion with status 130`. That is behaviorally safe but misidentifies the active phase. Track enough runner phase to emit setup-specific wording while preserving the provider-eval wording once launch begins.

## Acceptance criteria

- [ ] SIGINT before Promptfoo launch writes a setup-specific terminal diagnostic and status.json reason without claiming a Promptfoo eval had started.
- [ ] SIGINT after provider launch retains the existing Promptfoo/provider-eval interruption wording and signal-derived exit status.
- [ ] Deterministic regressions cover both setup-phase and provider-phase diagnostic wording.

## Subtasks

## Notes / Log

- 2026-07-14: Scope boundary from 20260713-uf3e split: this ticket owns setup-versus-provider interruption wording only; 20260714-2xyd owns preparation lifecycle and stale-evidence handling.
- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
