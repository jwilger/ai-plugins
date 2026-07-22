---
title: Make final-review behavior checks measure the intended protocol reliably
blocked_by: []
blocks: []
tags: [development-discipline, evals, final-review]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Final-review provider evals currently fail answers that enforce the intended coordinator safeguards because their rubrics also demand exhaustive recall of unrelated internal protocol details. This obscures real regressions and produces misleading red evidence.

## Context / Why

Observed while delivering MVJ4. The cases final-review-uses-mcp-for-enforced-review-coordination and final-review-rejects-forged-or-stale-session-state rejected broadly correct answers with scores around 0.5–0.825. Preserve hard guards for same-session identity, immutable coordinator state, authoritative MCP decisions, and terminal success, while separating those behaviors from exhaustive protocol-recall coverage.

## Acceptance criteria

- [ ] Provider-enabled answers pass when they demonstrate the intended core final-review safeguards without requiring exhaustive unrelated coordinator internals.
- [ ] The no-plugin baseline and value gate remain meaningful and prove useful plugin lift.

## Subtasks

## Notes / Log
