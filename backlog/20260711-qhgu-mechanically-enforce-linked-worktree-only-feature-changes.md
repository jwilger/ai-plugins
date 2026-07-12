---
title: Mechanically enforce linked-worktree-only feature changes
blocked_by: []
blocks: []
tags: [bug, worktrees, engineering-standards, development-discipline, guardrails]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Mechanically guard normal agent pre-edit entrypoints and Git integration operations against accidental feature work in the main coordination checkout while preserving legitimate coordination operations.

## Context / Why

A completed feature branch was fast-forwarded into the main coordination checkout. Existing pre-commit/pre-push hooks act too late to prevent direct edits or local integration. Use the repository's local single-owner threat model: prevent ordinary agent/operator mistakes at supported workflow and Git entrypoints, but do not attempt OS-level prevention of an intentional local-owner bypass or arbitrary external editor. Preserve existing user changes, allow documented coordination operations, and provide a narrow explicit exception rather than a broad disable switch.

## Acceptance criteria

- [ ] Feature-edit and integration workflows have a mechanical guard that detects use of the main coordination checkout before repository state is changed.
- [ ] The guard preserves documented coordination operations and provides a clear worktree remediation path when it blocks an action.
- [ ] Supported agent workflows run a checkout guard before edit tools mutate project files, and Git integration guards cover the normal local merge, fast-forward, pull, commit, and push paths.
- [ ] A documented allowlist preserves coordination operations such as fetch, worktree management, task-branch maintenance, and hook installation without permitting ordinary feature integration in main.
- [ ] Any exception is narrow, case-by-case, visible in diagnostics, and does not become a persistent global bypass.

## Subtasks

## Notes / Log
