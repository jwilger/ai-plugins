---
title: Locked expense-report benchmark fixture
blocked_by: []
blocks: []
tags: [evals, benchmark, rust, fixture, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Independently review the dependency-free locked Rust expense-report fixture split from cb43.

## Context / Why

Final-review split from 20260719-cb43 at diff hash b190d81690f3657f5230580fb083b666e86c8237. Scope is only evals/benchmarks/downstream-code-quality/fixtures/expense-report.

## Acceptance criteria

- [ ] Provide a dependency-free locked Rust fixture with a deterministic validate baseline.
- [ ] Include behavior-focused baseline coverage and repository-local candidate guidance.
- [ ] Keep build, cache, home, and temporary artifacts ignored inside the fixture.
- [ ] Complete final review on the isolated diff with current verification evidence.

## Subtasks

## Notes / Log
