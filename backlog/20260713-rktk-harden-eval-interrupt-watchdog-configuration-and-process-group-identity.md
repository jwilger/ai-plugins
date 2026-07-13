---
title: Harden eval interrupt watchdog configuration and process-group identity
blocked_by: []
blocks: []
tags: [evals, signals, process-management, hardening, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Validate interrupt-grace configuration and prevent delayed watchdog escalation from targeting a reused numeric process-group ID.

## Context / Why

Deferred MINOR findings from formal review of 20260712-rmgc. EVAL_INTERRUPT_GRACE is passed directly to sleep, so an invalid/non-finite override can disable bounded INT→TERM→KILL escalation. The watchdog also retains only a numeric PGID across grace sleeps; although reuse within the short local-tool window is unlikely, lifecycle identity should be anchored or escalation canceled when the original group is gone.

## Acceptance criteria

## Subtasks

## Notes / Log
