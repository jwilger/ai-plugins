---
title: Isolated workspace matrix preparation
blocked_by: []
blocks: []
tags: [evals, benchmark, workspace, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Independently review owned temporary workspace creation and exact matrix validation split from cb43.

## Context / Why

Final-review split from 20260719-cb43 at diff hash b190d81690f3657f5230580fb083b666e86c8237. Primary scope: manifest.cjs and prepare-code-quality-workspaces.mjs; direct declarative/fixture/tree-hash dependencies may be included only as supporting context, not expanded ownership.

## Acceptance criteria

- [ ] Prepare a fresh clean Git repository for every selected case, sample, and condition.

## Subtasks

## Notes / Log
