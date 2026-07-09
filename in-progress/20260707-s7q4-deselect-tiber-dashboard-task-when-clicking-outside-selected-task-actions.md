---
title: Deselect Tiber dashboard task when clicking outside selected-task actions
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

- 2026-07-09: Implemented dashboard outside-click deselection on signed branch origin/tiber-dashboard-deselect-click at 0003423. Validation passed: just ci, tiber validate --fix, scripts/evals/run.sh --dry-run, git diff --check. PR creation is waiting on final-review fresh-subagent capacity; subagent spawn currently fails with thread limit reached.
- 2026-07-09: Revalidated branch origin/tiber-dashboard-deselect-click at 0003423 from the linked worktree. Full nix develop -c just ci passed, including Rust tests, dashboard smoke, mutation tests, release completeness, and Bats. node scripts/evals/build-site.mjs wrote site/evals/index.html with no tracked diff. PR creation is still blocked by the required final-review fresh-subagent gate failing with 'agent thread limit reached'. Provider-backed evals still need explicit approval because they export repository/plugin content to external providers.
