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

- [ ] A single terminal SIGINT terminates the eval runner and the complete timeout, Promptfoo, Claude/Codex SDK, and descendant process tree within a bounded grace period.
- [ ] Interrupted runs return a signal-derived nonzero status, write interrupted status metadata, retain partial artifacts as designed, and never invoke Promptfoo sharing.
- [ ] A regression test sends a real SIGINT to the production-like foreground/process-group topology and proves no child or grandchild survives.

## Subtasks

## Notes / Log
