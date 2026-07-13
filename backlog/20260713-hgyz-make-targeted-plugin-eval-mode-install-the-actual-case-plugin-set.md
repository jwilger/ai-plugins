---
title: Make targeted-plugin eval mode install the actual case plugin set
blocked_by: []
blocks: []
tags: [evals, plugin-modes, measurement-validity, codex, claude, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Derive targeted-plugin provider composition from the selected behavior cases so targeted and full-marketplace rows are meaningfully distinct without a manual override.

## Context / Why

Pre-existing MAJOR found while reviewing 20260709-spx8 docs. scripts/evals/run.sh defaults EVAL_TARGETED_PLUGINS to every Codex marketplace plugin, while Claude targeted/full providers both load every Claude plugin; load-harness-cases.cjs does not alter installation. Default targeted and full rows are therefore compositionally identical despite distinct labels. Build the targeted set from selected case metadata for Codex and define an honest Claude equivalent or remove the duplicate label, with dashboard/config regressions.

## Acceptance criteria

## Subtasks

## Notes / Log
