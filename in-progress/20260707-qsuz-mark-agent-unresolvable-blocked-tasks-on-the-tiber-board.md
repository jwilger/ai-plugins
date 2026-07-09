---
title: Mark agent-unresolvable blocked tasks on the Tiber board
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

- 2026-07-07: Requirement detail: Tiber needs an explicit way to mark a task as blocked when the blocker is not resolvable by the agent. The board should show this visually, and the task detail view should show the blocking reason. This should not be used for ordinary waiting on PR checks or human approval; those should remain represented by PR/MR state rather than the unresolvable-blocked marker.
- 2026-07-09: Implemented on origin/tiber-unresolvable-blocked at d7a35e4. Validation passed: focused CLI/MCP/dashboard tests; rebuilt Tiber release artifacts; nix develop -c just ci; plugins/tiber/bin/tiber validate --fix; nix develop -c scripts/evals/run.sh --dry-run; git diff --check; plugin-eval analyze plugins/tiber/skills/tiber --format markdown scored 100/100. PR creation is waiting on the required final-review fresh-subagent gate.
