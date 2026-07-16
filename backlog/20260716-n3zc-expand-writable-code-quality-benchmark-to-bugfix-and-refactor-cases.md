---
title: Expand writable code-quality benchmark to bugfix and refactor cases
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Extend the safe writable Codex benchmark beyond the Rust feature pilot with representative bugfix and refactor scenarios in a second stack.

## Context / Why

Deferred from ticket 20260715-n6bs so the first evidence can focus on a safely contained skills-only Rust pilot. Reuse its credential, containment, provenance, failure taxonomy, trusted rebuild, and sanitized-artifact controls rather than creating a parallel harness.

## Acceptance criteria

- [ ] Adds realistic bugfix and refactor fixtures in TypeScript or Python with public-surface deterministic verifiers.
- [ ] Each case declares the case-relevant targeted skills and compares no skills, targeted skills, and all marketplace skills under the pilot containment boundary.
- [ ] Scoring covers behavior, regression tests, strict type/lint/format gates, diff scope, safety, and a calibrated semantic architecture rubric only where deterministic checks are insufficient.

## Subtasks

## Notes / Log
