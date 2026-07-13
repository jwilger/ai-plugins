---
title: Keep README plugin catalog versions synchronized with manifests
blocked_by: []
blocks: []
tags: [marketplace, documentation, versions, validation, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the README harness catalog versions derive from or validate against plugin manifests so plugin releases cannot leave user-facing version rows stale.

## Context / Why

MINOR caused finding from the 20260709-spx8 review. README currently advertises pre-bump versions for agentic-systems-engineering, advisor, babysit-pr, and tiber even though their synchronized manifests/marketplace entries have advanced. Add an automated sync/validation path and repair the rows in that focused ticket.

## Acceptance criteria

- [ ] Version-specific plugin troubleshooting documentation, including Tiber Codex cache paths, remains synchronized with the released manifest version or uses manifest-versioned wording that cannot drift.

## Subtasks

## Notes / Log

- 2026-07-13: 20260709-spx8 final review deferred a caused MINOR: Tiber's launcher now targets cache version 0.9.0 while the plugin README still names 0.6.1. Covered by the version-documentation synchronization criterion.
