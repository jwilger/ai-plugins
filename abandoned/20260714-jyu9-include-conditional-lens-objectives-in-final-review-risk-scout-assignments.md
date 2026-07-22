---
title: Explain custom review checks to the risk scout
blocked_by: []
blocks: []
tags: [development-discipline, final-review, risk-planning, mcp, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Include the purpose of every custom review check in the risk scout’s assignment. This lets the scout evaluate specialized migration, domain, and safety risks from their intended meaning instead of seeing only an unexplained identifier.

## Context / Why

## Acceptance criteria

- [ ] Risk-scout assignments include the validated ID and description/objective for every configured conditional lens in both the bound assessment context and prompt.
- [ ] Changing a conditional lens description after assessment creation invalidates the assessment binding instead of silently accepting stale risk evidence.
- [ ] Focused Rust and public JSON-RPC tests cover conditional-lens objective delivery and binding validation.

## Subtasks

## Notes / Log

- 2026-07-14: 2026-07-14: Consolidated from a lightweight review of active policy ticket 20260713-rygd. The scout currently receives custom lens IDs but not their descriptions; classified MINOR and deferred under the global backlog policy.
- 2026-07-22: 2026-07-22 curation rejection: Lower value relative to cost than the retained cross-project final-review identity/restart blocker. This is readiness, fixture, scale, cleanup, or protocol-quality follow-up rather than the repeated root-cause delivery failure; no shadow ticket is retained.
