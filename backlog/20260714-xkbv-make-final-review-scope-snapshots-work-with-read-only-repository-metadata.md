---
title: Make final-review scope snapshots work with read-only repository metadata
blocked_by: []
blocks: []
tags: [development-discipline, final-review, git, snapshots, permissions, operability, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Create immutable final-review scope snapshots without writing objects or refs into the reviewed repository's Git metadata, while preserving exact diff binding and actionable recovery.

## Context / Why

A caused MINOR operability finding from 20260713-rygd showed that mandatory scope snapshot creation runs git add, write-tree, and commit-tree against the reviewed repository's object store. Managed workspaces commonly permit working-tree writes while keeping .git metadata read-only, causing final_review.assess_risk to fail before the mandatory scout with no actionable recovery guidance. Use an isolated temporary object database or equivalent immutable snapshot mechanism without weakening scope identity.

## Acceptance criteria

- [ ] Scope snapshot creation does not require writing objects or refs into the reviewed repository's Git metadata; use an isolated temporary object database or an equivalent immutable snapshot mechanism.
- [ ] Preserve exact baseline, changed-file, tracked/untracked-content, and diff-hash binding.
- [ ] A read-only .git with writable worktree is covered by a focused regression.

## Subtasks

## Notes / Log

- 2026-07-14: Consolidation check for 20260713-rygd found no duplicate. Priority evidence: high workflow value, medium-to-high impact, observed likelihood in managed workspaces, and moderate implementation cost.
