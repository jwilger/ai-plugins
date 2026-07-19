---
title: Run fast CI checks before expensive ones
blocked_by: []
blocks: []
tags: [ci, engineering-standards, scaffold, workflow-design]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Update generated CI workflows so inexpensive checks fail quickly, independent checks run in parallel, and costly checks start only after the fast gates pass. Final reporting should still clearly show what failed.

## Context / Why

Implementation notes: engineering-standards:scaffold already owns generic CI generation and its playbook, so do not create a competing general CI skill unless trigger evidence proves a distinct user-facing responsibility. Add the reusable staged fast-fail model there, including always-run aggregation/reporting and deliberate cancellation behavior. Add platform-specific references only where GitHub Actions, GitLab CI, Forgejo, or another system has irreducible syntax or capability differences.

## Acceptance criteria

- [ ] Existing CI-related plugin and skill guidance is audited, and the new responsibility boundaries avoid duplication.
- [ ] Generic guidance expresses CI as dependency stages, runs independent same-stage checks in parallel, and gates slow or expensive stages on successful fast checks.
- [ ] Tool-specific skills or references are added only where a CI platform’s syntax or capabilities cannot be expressed clearly by the generic skill.
- [ ] The existing scaffold skill and playbook are extended rather than duplicating their trigger and responsibility, unless a separate skill is justified by explicit trigger evidence.
- [ ] Guidance defines fast and slow dependency stages, parallel same-stage jobs, slow-stage gating, always-run aggregation/reporting, and cancellation semantics.
- [ ] Behavior fixtures exercise a cheap failure that prevents expensive work, independent checks that remain parallel, and a final aggregate status that still reports actionable failures.

## Subtasks

## Notes / Log
