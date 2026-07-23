---
title: Make Tiber safe to add to established repositories
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

## Context / Why

## Acceptance criteria

- [ ] Initialization refuses to create Tiber task state when an existing root `.tasks` system would create a parallel board, and reports actionable integration guidance without mutation.
- [ ] Repository scaffold dry-run identifies whether the active hook manager will dispatch Tiber's task-closing hook and reports conflicts instead of installing inert automation.
- [ ] Generated task-closing workflow uses a pinned intended revision, declares least-required explicit permissions, and supports or clearly refuses repositories whose publication policy requires signed commits.
- [ ] Existing dry-run, conflict, existing-target, and atomic preservation guarantees remain covered by tests.
- [ ] Documentation explains how established repositories can safely adopt Tiber without replacing or bypassing their current task, hook, workflow, permission, or signing policies.

## Subtasks

## Notes / Log

- 2026-07-23: Admitted from GitHub issue #60 as the current cross-project Tiber adoption blocker. It is not a duplicate of closed issue #54: that issue covered an incompatible existing `tasks` ref, while this work covers a source-tree `.tasks` system and operational compatibility with existing hooks, workflows, permissions, and signed publication.
