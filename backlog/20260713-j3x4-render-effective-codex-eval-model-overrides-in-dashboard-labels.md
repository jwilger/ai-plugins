---
title: Render effective Codex eval model overrides in dashboard labels
blocked_by: []
blocks: []
tags: [evals, codex, dashboard, model-overrides, bug, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Persist and render the effective Codex model when environment overrides replace the configured default, instead of retaining a static Terra-labelled provider ID.

## Context / Why

Pre-existing MINOR found during 20260709-spx8. scripts/evals/generate-config.mjs labels Codex providers with the default model while CODEX_EVAL_MODEL may select Sol or Luna; scripts/evals/build-site.mjs trusts that label, so dashboards can misidentify overridden runs. Add model-neutral labels or persisted effective-model metadata plus regression coverage.

## Acceptance criteria

## Subtasks

## Notes / Log
