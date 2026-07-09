---
title: Make Tiber opaque to host repositories and agents
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-09: Validated and pushed signed branch origin/tiber-opaque-host-repos at 4c030a6. PR creation is waiting on final-review fresh-subagent capacity; subagent spawn currently fails with thread limit reached.
- 2026-07-09: Revalidated branch origin/tiber-opaque-host-repos at 4c030a6 from the linked worktree. Full nix develop -c just ci passed after cleaning stale shared Cargo artifacts for tiber-git; node scripts/evals/build-site.mjs wrote site/evals/index.html with no tracked diff; plugin-eval static analysis for plugins/tiber/skills/tiber and plugins/tiber/skills/new-task both scored 100/100 with only informational coverage-artifact notes. PR creation is still blocked by the required final-review fresh-subagent gate failing with 'agent thread limit reached'. Provider-backed just evals was attempted but rejected by the approval reviewer because it would export repository/plugin content to external eval providers; explicit user approval is needed before running it.
