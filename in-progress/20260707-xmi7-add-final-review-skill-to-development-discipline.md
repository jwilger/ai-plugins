---
title: Add final review skill to development-discipline
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

Add a final-review skill to the development-discipline plugin. The skill should run locally before creating any pull request or merging to main. It should review a change scope against a baseline, defaulting to origin/main but respecting user-specified scopes such as uncommitted changes, changes since a release tag, or another explicit base. Each review cycle should spawn fresh-context subagents for each review lens and iteration, address valid findings or write a technical defense for intentionally not addressing them, then repeat until three consecutive iterations produce no issues from any lens.

## Context / Why

## Acceptance criteria

- [ ] The development-discipline plugin includes a final-review skill with trigger guidance for local use before PR creation or merge-to-main.
- [ ] The skill defines default review lenses: correctness/behavior, tests/verification, security/safety, architecture/maintainability, user experience/operability, release/integration readiness, and agent-instruction quality for plugin or skill changes.
- [ ] The review loop uses fresh-context subagents for every lens and iteration, communicates prior defenses to relevant lenses, and terminates only after three consecutive issue-free iterations across all lenses.
- [ ] The skill documents how to derive the review diff from origin/main by default and from user-specified baselines such as uncommitted changes or changes since the last release tag.
- [ ] The review lenses include an explicit production-risk/footgun pass, or equivalent lens coverage, that looks for latent footguns, fragile defaults, hidden operational traps, and data-access patterns that pass in dev/test/staging but fail at expected production scale or under burst/DOS-like load.
- [ ] The final-review skill keeps default lenses repository-agnostic and adaptable across codebases; domain-specific lenses such as agent-instruction quality are conditional examples for relevant changes, not hardcoded requirements for every repository.
- [ ] The final-review behavior is covered by development-discipline eval cases.

## Subtasks

## Notes / Log
