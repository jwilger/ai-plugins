---
title: Keep README plugin versions consistent with released manifests
blocked_by: []
blocks: []
tags: [marketplace, documentation, versions, validation, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Automatically check or update the plugin versions shown in README catalog tables and troubleshooting guidance. User-facing documentation should not advertise an older version than the authoritative release manifests.

## Context / Why

MINOR caused finding from the 20260709-spx8 review. README currently advertises pre-bump versions for agentic-systems-engineering, advisor, babysit-pr, and tiber even though their synchronized manifests/marketplace entries have advanced. Add an automated sync/validation path and repair the rows in that focused ticket.

## Acceptance criteria

- [ ] Version-specific plugin troubleshooting documentation, including Tiber Codex cache paths, remains synchronized with the released manifest version or uses manifest-versioned wording that cannot drift.
- [ ] Both README plugin catalog tables match the authoritative per-harness manifest versions for every listed plugin, including agentic-systems-engineering, advisor, babysit-pr, and tiber.

## Subtasks

## Notes / Log

- 2026-07-13: 20260709-spx8 final review deferred a caused MINOR: Tiber's launcher now targets cache version 0.9.0 while the plugin README still names 0.6.1. Covered by the version-documentation synchronization criterion.
- 2026-07-14: 2026-07-14 formal final-review pass 1 for 20260709-spx8 reconfirmed the README marketplace catalog/version mismatch. Deferred as MINOR; covered by this ticket's existing cross-harness catalog synchronization criteria.
- 2026-07-22: 2026-07-22 curation rejection: Release, tooling, or maintenance convenience with lower current blocking impact and value-to-cost than the retained defects and security update. Rejected without an overflow or shadow backlog.
