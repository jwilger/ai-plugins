---
title: Open one reusable dashboard for each project
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

Starting a Tiber dashboard from an agent session should open the correct project without port conflicts or duplicate server processes. Choose an available local port for a genuinely new dashboard, but detect and reuse an already-running dashboard when a compacted or resumed session repeats the launch request.

## Context / Why

Multiple projects need dashboards on one machine, while repeated launch instructions after language-model session compaction can accidentally start more servers and open more browser windows. The result should be one discoverable dashboard instance per project, explicit fixed-port support where needed, and browser opening that is deliberate and does not repeat merely because session context was compacted. This readmits and broadens the previously rejected yqyy candidate based on newly observed recurring launch behavior.

## Acceptance criteria

- [x] Starting a dashboard without a fixed port selects an available local port and prints the complete URL.
- [x] Repeating the launch for the same project, including after an agent session compacts or resumes, reuses the healthy existing instance instead of starting another dashboard.
- [x] Dashboard discovery distinguishes projects so different repositories can run simultaneously without sharing an instance or conflicting on a port.
- [x] Browser opening is explicit or occurs only for a genuinely new launch, so repeated agent instructions do not keep opening windows.
- [x] Users can still request a specific port, and an unavailable requested port fails with a clear diagnostic.

## Subtasks

## Notes / Log
