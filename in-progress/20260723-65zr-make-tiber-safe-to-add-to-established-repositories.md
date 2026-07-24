---
title: Make Tiber safe to add to established repositories
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Established repositories can already have task data, hooks, workflow conventions, permissions, and signing requirements. Tiber setup must recognize and preserve those systems instead of creating a parallel board or scaffold that appears installed but cannot operate safely.

## Context / Why

GitHub issue 60 reports that initialization can create a new tasks branch even when a root .tasks directory already represents an existing task system. Scaffolding can also emit a post-commit hook without proving that the active hook manager will dispatch it, and generate a task-closing workflow without pinning the intended revision, declaring explicit permissions, or supporting repositories that require signed publication. The desired outcome is setup that detects these conflicts before mutation, explains safe integration choices, and generates only operational automation that respects existing task, hook, workflow, permission, and signing policies. Implementation notes: preserve dry-run and conflict safety; distinguish source-tree .tasks from Git-object-backed Tiber state; verify active hooksPath or hook-manager dispatch; pin generated workflow revisions; declare least-required permissions; and support or clearly refuse signed-publication requirements.

## Acceptance criteria

- [x] Initialization refuses to create Tiber task state when an existing root `.tasks` system would create a parallel board, and reports actionable integration guidance without mutation.
- [x] Repository scaffold dry-run identifies whether the active hook manager will dispatch Tiber's task-closing hook and reports conflicts instead of installing inert automation.
- [x] Generated task-closing workflow uses a pinned intended revision, declares least-required explicit permissions, and supports or clearly refuses repositories whose publication policy requires signed commits.
- [ ] Existing dry-run, conflict, existing-target, and atomic preservation guarantees remain covered by tests.
- [ ] Documentation explains how established repositories can safely adopt Tiber without replacing or bypassing their current task, hook, workflow, permission, or signing policies.

## Subtasks

## Notes / Log

- 2026-07-23: Admitted from GitHub issue #60 as the current cross-project Tiber adoption blocker. It is not a duplicate of closed issue #54: that issue covered an incompatible existing `tasks` ref, while this work covers a source-tree `.tasks` system and operational compatibility with existing hooks, workflows, permissions, and signed publication.
