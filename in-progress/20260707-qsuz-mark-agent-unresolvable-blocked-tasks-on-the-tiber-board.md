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
- 2026-07-09: Final-review iteration 1 found issues and they were fixed: tiber next now skips agent-blocked tasks until cleared; MCP tests cover agent_blocked_reason schema/set/clear; release-binary smoke covers host dist binary; docs warn not to store secrets and explain clearing; done tasks no longer show stale blocked badges; root README version references are 0.7.0. Amended commit is 18105eb on origin/tiber-unresolvable-blocked-reviewed; original origin/tiber-unresolvable-blocked remains stale because GitHub branch rules reject force-pushes.
- 2026-07-09: Final-review iteration 3 found additional issues and they were fixed in dffa31e on origin/tiber-unresolvable-blocked-reviewed: the dashboard card now visibly shows the blocked reason for keyboard/touch users, and MCP tiber.update metadata now carries the agent-unresolvable/secret-handling guidance. Validation passed again: nix develop -c just ci; plugins/tiber/bin/tiber validate --fix; nix develop -c scripts/evals/run.sh --dry-run; git diff --check; plugin-eval analyze plugins/tiber/skills/tiber --format markdown scored 100/100.
- 2026-07-09: Final-review clean iteration 1 found a valid tests-verification issue: CI did not prove release artifacts were rebuilt from current source. Fixed in 535b03b on origin/tiber-unresolvable-blocked-reviewed by adding scripts/check-tiber-release-fresh.sh, wiring just ci to rebuild all release artifacts, and adding Bats coverage for clean-checkout failure and dirty-input local skip behavior. Validation passed again: nix develop -c just ci; plugins/tiber/bin/tiber validate --fix; nix develop -c scripts/evals/run.sh --dry-run; git diff --check; plugin-eval analyze plugins/tiber/skills/tiber --format markdown scored 100/100.
