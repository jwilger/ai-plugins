---
title: Make Ctrl-C terminate the full provider-eval process tree
blocked_by: []
blocks: []
tags: [evals, signals, process-management, bug, major]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Ensure one Ctrl-C promptly terminates the runner, GNU timeout wrapper, Promptfoo, provider SDK processes, and their descendants without sharing partial results.

## Context / Why

Live diagnosis showed scripts/evals/run.sh in the terminal foreground PGID while `timeout` created a separate PGID for Promptfoo and Codex descendants. The terminal delivered SIGINT only to the runner shell, which has no INT/TERM trap or process-group forwarding, so the eval continued until the terminal was closed. Existing interrupt tests only use a fake command that exits 130; they do not deliver a real SIGINT across the production process topology. This is a pre-existing MAJOR operability/cost defect because users cannot reliably stop paid provider calls.

## Acceptance criteria

- [x] A single terminal SIGINT terminates the eval runner and the complete timeout, Promptfoo, Claude/Codex SDK, and descendant process tree within a bounded grace period.
- [x] Interrupted runs return a signal-derived nonzero status, write interrupted status metadata, retain partial artifacts as designed, and never invoke Promptfoo sharing.
- [ ] A regression test sends a real SIGINT to the production-like foreground/process-group topology and proves no child or grandchild survives.

## Subtasks

## Notes / Log

- 2026-07-13: Completed in signed commit bf13dae41c7612a93b077039a5de351e81a0f020 and pushed to main. Verification: focused launch-race regression passed; relevant eval-runner suites passed 34/34; full local `just ci` passed with 44 mutants (38 caught, 6 unviable) and 201 Bats tests; final review reached clean_streak 3 on unchanged scope hash 86c4d98e837218119bfb239b687f2ea289baceab with no unresolved findings; GitHub CI run https://github.com/jwilger/ai-plugins/actions/runs/29259885955 completed successfully. Provider-backed evals were intentionally not run per user direction until ticket 20260709-spx8 migrates defaults to GPT-5.6 Sol/Terra/Luna.
