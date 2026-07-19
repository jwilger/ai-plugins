---
title: Let final review work when Git metadata is read-only
blocked_by: []
blocks: []
tags: [development-discipline, final-review, git, snapshots, permissions, operability, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Allow final review to create an exact, stable snapshot of the proposed change without writing into the repository’s protected Git metadata. Managed workspaces should be reviewable without weakening the link between the review and the code being reviewed.

## Context / Why

Implementation notes:\n\nA caused MINOR operability finding from 20260713-rygd showed that mandatory scope snapshot creation runs git add, write-tree, and commit-tree against the reviewed repository's object store. Managed workspaces commonly permit working-tree writes while keeping .git metadata read-only, causing final_review.assess_risk to fail before the mandatory scout with no actionable recovery guidance. Use an isolated temporary object database or equivalent immutable snapshot mechanism without weakening scope identity.

## Acceptance criteria

- [ ] Initial and delta scope snapshot creation does not write objects, refs, indexes, or other metadata into the reviewed repository's Git directory and succeeds when the worktree is writable but .git is read-only.
- [ ] The replacement mechanism preserves the exact baseline, changed-file inventory, tracked, staged, unstaged, and untracked content coverage, immutable scope reference, and diff-hash binding.
- [ ] Focused Rust and public JSON-RPC regressions exercise a writable worktree with read-only repository metadata for both initial assessment and delta reassessment.
- [ ] Permission or snapshot-construction failure emits an actionable diagnostic and supported recovery path without silently bypassing or weakening final-review enforcement.
- [ ] Temporary snapshot state is isolated, bounded, and cleaned safely without deleting user or repository data.

## Subtasks

## Notes / Log

- 2026-07-14: Consolidation check for 20260713-rygd found no duplicate. Priority evidence: high workflow value, medium-to-high impact, observed likelihood in managed workspaces, and moderate implementation cost.
