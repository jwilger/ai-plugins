---
title: Block pushes until final-review completes three clean passes
blocked_by: []
blocks: []
tags: [bug, development-discipline, final-review, release-enforcement, git]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a stable final-review state checker and mechanically gate publication so pushes and supported merge paths proceed only after three consecutive clean iterations for the exact current change.

## Context / Why

The final-review MCP already tracks three-clean-iteration state, prior defenses, and a scope hash, but current hooks do not consume that state. A partial first iteration was followed by a push. Provide a hook/CI-consumable check bound to the repository, worktree, base, updated ref, and exact current diff or commit hash. Local pre-push and remote PR/merge enforcement are distinct surfaces and both must be covered or their limits made explicit. Fail closed when required state is missing, stale, mismatched, or unavailable. Exempt only infrastructure that must publish without final review, especially the Git-backed Tiber tasks branch, using a narrow documented rule.

## Acceptance criteria

- [ ] A push or merge of an in-scope change is blocked unless the authoritative final-review state records three consecutive clean iterations for the exact current diff hash.
- [ ] The block message identifies the missing, stale, or mismatched review state and gives the developer a clear command or workflow to complete it.
- [ ] A stable read/check interface exposes authoritative final-review completion and binds it to the repository/worktree, base, updated ref, and exact reviewed hash.

## Subtasks

## Notes / Log
