---
title: Add staged fast-fail CI gating to engineering-standards scaffold
blocked_by: []
blocks: []
tags: [ci, engineering-standards, scaffold, workflow-design]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Extend the existing engineering-standards scaffold guidance so generated CI uses dependency stages: cheap validation first, parallel independent checks within a stage, and expensive checks only after fast gates pass.

## Context / Why

engineering-standards:scaffold already owns generic CI generation and its playbook, so do not create a competing general CI skill unless trigger evidence proves a distinct user-facing responsibility. Add the reusable staged fast-fail model there, including always-run aggregation/reporting and deliberate cancellation behavior. Add platform-specific references only where GitHub Actions, GitLab CI, Forgejo, or another system has irreducible syntax or capability differences.

## Acceptance criteria

- [ ] Existing CI-related plugin and skill guidance is audited, and the new responsibility boundaries avoid duplication.
- [ ] Generic guidance expresses CI as dependency stages, runs independent same-stage checks in parallel, and gates slow or expensive stages on successful fast checks.
- [ ] Tool-specific skills or references are added only where a CI platform’s syntax or capabilities cannot be expressed clearly by the generic skill.
- [ ] The existing scaffold skill and playbook are extended rather than duplicating their trigger and responsibility, unless a separate skill is justified by explicit trigger evidence.

## Subtasks

## Notes / Log
