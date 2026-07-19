---
title: Let each Tiber dashboard choose an available port
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Allow Tiber dashboards for several projects to run on the same machine without port conflicts. When no port is specified, the dashboard should choose an available local port and clearly print the URL it selected.

## Context / Why

Implementation notes: Keep an explicit port option for scripts, bookmarks, and other workflows that need a predictable address. Automatic selection should ask the operating system for an available port instead of guessing from a fixed range, and startup errors should identify any explicitly requested port that is already in use.

## Acceptance criteria

- [ ] Starting the dashboard without a port chooses an available local port and prints the complete URL.
- [ ] Dashboards for multiple projects can start at the same time on one machine without using the same port.
- [ ] A user can still request a specific port for predictable automation.

## Subtasks

## Notes / Log
