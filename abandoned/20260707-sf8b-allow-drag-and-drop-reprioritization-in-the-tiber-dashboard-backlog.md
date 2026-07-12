---
title: Allow drag-and-drop reprioritization in the Tiber dashboard backlog
blocked_by: []
blocks: []
tags: []
---

## Summary

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-09: Implemented dashboard backlog drag-and-drop reprioritization on signed branch origin/tiber-dashboard-drag-drop at 4d55fb9. Validation passed: just ci, tiber validate --fix, scripts/evals/run.sh --dry-run, git diff --check. PR creation is waiting on final-review fresh-subagent capacity; subagent spawn currently fails with thread limit reached.
- 2026-07-09: Revalidated branch origin/tiber-dashboard-drag-drop at 4d55fb9 from the linked worktree. Initial just ci hit stale shared Cargo artifacts in tiber-git; after cargo clean -p tiber-git, the focused init test passed and full nix develop -c just ci passed. node scripts/evals/build-site.mjs wrote site/evals/index.html with no tracked diff. PR creation is still blocked by the required final-review fresh-subagent gate failing with 'agent thread limit reached'. Provider-backed evals still need explicit approval because they export repository/plugin content to external providers.
- 2026-07-12: 2026-07-11: Reopened after live dashboard testing. Reorder route works, but the page reload makes changes appear ineffective and native dragging cannot reach off-screen targets. Direction: replace reload-based reconciliation with a versioned board snapshot and client-side state renderer; render an insertion gap between cards and add column edge auto-scroll. Docs/task routes remain server-rendered.
