---
title: Clean up old final-review patch artifacts safely
blocked_by: []
blocks: []
tags: [development-discipline, final-review, cleanup, operability, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add bounded cleanup for stored final-review patch files so they do not accumulate indefinitely. Keep any artifact still needed to identify or resubmit a pending review, and never remove user or repository data.

## Context / Why

MINOR finding from the risk-proportionate final-review lightweight pass. Artifacts are local, content-addressed, and created only for patches above the inline threshold; accumulation is low-likelihood operational clutter rather than a blocking security or safety issue.

## Acceptance criteria

## Subtasks

## Notes / Log
