---
title: Make final-review scope snapshots work with read-only repository metadata
blocked_by: []
blocks: []
tags: [development-discipline, final-review, operability, git, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Remove the mandatory final-review scout's dependency on writable repository Git metadata by isolating scope-snapshot objects while preserving exact diff binding.

## Context / Why

Final-review operability finding from 20260713-rygd. This is distinct from 20260714-gb9s pathspec/evidence-size bounds, 20260714-hmwe ambient signing/hooks, and 20260714-iv3g delta-artifact cleanup; none addresses mandatory snapshot writes to a read-only repository object store.

## Acceptance criteria

- [ ] Scope snapshot creation does not require writing objects or refs into the reviewed repository's Git metadata; use an isolated temporary object database or an equivalent immutable snapshot mechanism.
- [ ] Preserve exact baseline, changed-file, tracked/untracked-content, and diff-hash binding.
- [ ] A read-only .git with writable worktree is covered by a focused regression.

## Subtasks

## Notes / Log
