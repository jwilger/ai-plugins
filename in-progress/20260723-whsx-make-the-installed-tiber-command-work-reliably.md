---
title: Make the installed Tiber command work reliably
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

Tiber's supported installation command can produce a launcher that does not run correctly. Repair the installation path so users can install and invoke Tiber without falling back to repository-relative commands.

## Context / Why

A broken installed command undermines the normal setup and upgrade experience and repeatedly forces users or agents to rediscover a workaround. This ticket should resolve the behavior represented by Tiber GitHub issue 59 and keep dry-run, conflict, and existing-target safety intact.

## Acceptance criteria

- [x] An installation applied to a supported target directory produces a Tiber command that launches successfully.
- [x] Dry-run output, existing-target refusal, documentation, and automated tests cover the repaired installation workflow.

## Subtasks

## Notes / Log

- 2026-07-23: Delivered by 5dc0735fa6dbdf80fe456887949fc85f899a2988; exact-SHA GitHub Actions run 30036919485 reached terminal success. Full repository checks and release artifact verification passed; full-marketplace canary passed 2/2 on unchanged rerun.
