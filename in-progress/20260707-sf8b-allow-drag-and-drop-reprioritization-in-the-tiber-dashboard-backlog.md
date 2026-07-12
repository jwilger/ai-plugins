---
title: Allow drag-and-drop reprioritization in the Tiber dashboard backlog
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

- 2026-07-09: Implemented dashboard backlog drag-and-drop reprioritization on signed branch origin/tiber-dashboard-drag-drop at 4d55fb9. Validation passed: just ci, tiber validate --fix, scripts/evals/run.sh --dry-run, git diff --check. PR creation is waiting on final-review fresh-subagent capacity; subagent spawn currently fails with thread limit reached.
- 2026-07-09: Revalidated branch origin/tiber-dashboard-drag-drop at 4d55fb9 from the linked worktree. Initial just ci hit stale shared Cargo artifacts in tiber-git; after cargo clean -p tiber-git, the focused init test passed and full nix develop -c just ci passed. node scripts/evals/build-site.mjs wrote site/evals/index.html with no tracked diff. PR creation is still blocked by the required final-review fresh-subagent gate failing with 'agent thread limit reached'. Provider-backed evals still need explicit approval because they export repository/plugin content to external providers.
