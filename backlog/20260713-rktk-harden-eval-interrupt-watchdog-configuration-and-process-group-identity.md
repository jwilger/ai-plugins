---
title: Make interrupted evaluation cleanup target only the original run
blocked_by: []
blocks: []
tags: [evals, signals, process-management, hardening, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Validate the grace period used when stopping an evaluation and ensure delayed cleanup signals cannot reach a later, unrelated process that happens to reuse the same numeric process identifier.

## Context / Why

Deferred MINOR findings from formal review of 20260712-rmgc. EVAL_INTERRUPT_GRACE is passed directly to sleep, so an invalid/non-finite override can disable bounded INT→TERM→KILL escalation. The watchdog also retains only a numeric PGID across grace sleeps; although reuse within the short local-tool window is unlikely, lifecycle identity should be anchored or escalation canceled when the original group is gone.

## Acceptance criteria

- [ ] EVAL_INTERRUPT_GRACE is parsed once as a finite supported duration and invalid values fail fast with a clear configuration error before launching Promptfoo.
- [ ] Focused tests cover invalid grace values and prove they cannot leave an interrupting runner blocked.
- [ ] Delayed TERM/KILL escalation remains bound to the original eval process group or is canceled on original-group completion, without signaling a later unrelated PGID.

## Subtasks

## Notes / Log
