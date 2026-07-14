---
title: Reject dangling symlinks for GPT-5.6 benchmark workspaces
blocked_by: []
blocks: []
tags: [minor, evals, gpt-5.6, workspace-isolation, safety]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Reject every symlink at the configured GPT-5.6 benchmark workspace path, including dangling symlinks, before marker ownership or preparation logic can replace it.

## Context / Why

Lightweight review reproduced ordinary stale-state behavior where existsSync treats a dangling symlink as missing; prepare-gpt56-workspace.mjs then removes it and creates a marker-owned directory. Use lstat-style entry detection so the unowned-path refusal covers dangling symlinks while preserving the intended local single-owner threat model.

## Acceptance criteria

## Subtasks

## Notes / Log
