---
title: Build writable downstream code-quality benchmarks for Codex plugins
blocked_by: []
blocks: []
tags: [codex, evals, quality, major, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Build a safe, non-promotional writable Rust benchmark that measures whether Codex produces better downstream code with no skills, the declared quality-core skills, and all marketplace skills.

## Context / Why

Re-scoped after the first Promptfoo wiring security review. The immediate decision-quality need is directional evidence about skill-driven code quality, not broad multi-language or full executable-plugin coverage. Run the expense-report feature case with three samples per condition using skills-only projections, bounded execution, API-key-only auth, trusted source rebuilds, explicit failure taxonomy, and sanitized artifacts. Defer TypeScript bugfix/refactor expansion and true full-plugin runtime execution to follow-up tickets.

## Acceptance criteria

- [ ] Artifacts preserve machine-readable results, reviewer-facing output, latency, token usage, and cost without exposing secrets or private project data.

## Subtasks

## Notes / Log
