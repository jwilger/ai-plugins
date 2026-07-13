---
title: Distinguish eval setup interruption diagnostics from Promptfoo interruption
blocked_by: []
blocks: []
tags: [evals, signals, diagnostics, operability, minor, review-follow-up]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Report setup-phase Ctrl-C with phase-accurate terminal and status.json wording instead of claiming a Promptfoo eval was interrupted before Promptfoo starts.

## Context / Why

Final review found a MINOR operability issue in the new pre-Promptfoo signal path. SIGINT during ensure-node-deps.sh, generated configuration, or Codex-home preparation currently routes through finish_eval_interruption and persists `promptfoo eval was interrupted before completion with status 130`. That is behaviorally safe but misidentifies the active phase. Track enough runner phase to emit setup-specific wording while preserving the provider-eval wording once launch begins.

## Acceptance criteria

- [ ] SIGINT before Promptfoo launch writes a setup-specific terminal diagnostic and status.json reason without claiming a Promptfoo eval had started.
- [ ] SIGINT after provider launch retains the existing Promptfoo/provider-eval interruption wording and signal-derived exit status.

## Subtasks

## Notes / Log
