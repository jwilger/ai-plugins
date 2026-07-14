---
title: Include conditional lens objectives in final-review risk-scout assignments
blocked_by: []
blocks: []
tags: [development-discipline, final-review, risk-planning, mcp, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Pass every configured conditional lens description/objective into the final-review risk scout's diff-bound assignment so custom migration, domain, and safety risks can be classified from their intended meaning.

## Context / Why

## Acceptance criteria

- [ ] Risk-scout assignments include the validated ID and description/objective for every configured conditional lens in both the bound assessment context and prompt.
- [ ] Changing a conditional lens description after assessment creation invalidates the assessment binding instead of silently accepting stale risk evidence.
- [ ] Focused Rust and public JSON-RPC tests cover conditional-lens objective delivery and binding validation.

## Subtasks

## Notes / Log

- 2026-07-14: 2026-07-14: Consolidated from a lightweight review of active policy ticket 20260713-rygd. The scout currently receives custom lens IDs but not their descriptions; classified MINOR and deferred under the global backlog policy.
