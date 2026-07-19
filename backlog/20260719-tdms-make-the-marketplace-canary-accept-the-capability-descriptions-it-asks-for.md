---
title: Make the marketplace canary accept the capability descriptions it asks for
blocked_by: []
blocks: []
tags: [evals, canary, prompt-contract, codex]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Align the full-marketplace canary checker with its prompt so an accurate plugin capability can satisfy the check without requiring an undocumented literal skill name.

## Context / Why

The canary prompt asks each harness to name every marketplace plugin and provide at least one representative skill or capability. On two consecutive live runs, Codex named all eight plugins and accurately described eval-case-reporter as creating scrubbed, approval-gated eval reports. The checker still failed because it accepts only the literal skill directory name submit-eval-case. This creates false loading failures even when the response proves the plugin is available.

## Acceptance criteria

- [ ] The canary prompt and checker use the same definition of an acceptable representative skill or capability.

## Subtasks

## Notes / Log
