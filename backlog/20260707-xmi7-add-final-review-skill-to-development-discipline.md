---
title: Add final review skill to development-discipline
blocked_by: []
blocks: []
tags: []
---

## Summary

Add a final-review skill to the development-discipline plugin. The skill should run locally before creating any pull request or merging to main. It should review a change scope against a baseline, defaulting to origin/main but respecting user-specified scopes such as uncommitted changes, changes since a release tag, or another explicit base. Each review cycle should spawn fresh-context subagents for each review lens and iteration, address valid findings or write a technical defense for intentionally not addressing them, then repeat until three consecutive iterations produce no issues from any lens.

## Context / Why

## Acceptance criteria

- [ ] The development-discipline plugin includes a final-review skill with trigger guidance for local use before PR creation or merge-to-main.
- [ ] The skill defines default review lenses: correctness/behavior, tests/verification, security/safety, architecture/maintainability, user experience/operability, release/integration readiness, and agent-instruction quality for plugin or skill changes.
- [ ] The review loop uses fresh-context subagents for every lens and iteration, communicates prior defenses to relevant lenses, and terminates only after three consecutive issue-free iterations across all lenses.

## Subtasks

## Notes / Log
