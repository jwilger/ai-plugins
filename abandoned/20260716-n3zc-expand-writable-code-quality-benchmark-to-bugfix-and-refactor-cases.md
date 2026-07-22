---
title: Add bug-fix and refactoring cases to the writable code-quality benchmark
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Expand the safely contained writable benchmark beyond its first Rust feature case by adding realistic bug-fix and refactoring scenarios in TypeScript or Python. Reuse the existing safety, provenance, verification, and result-reporting controls.

## Context / Why

Deferred from ticket 20260715-n6bs so the first evidence can focus on a safely contained skills-only Rust pilot. Reuse its credential, containment, provenance, failure taxonomy, trusted rebuild, and sanitized-artifact controls rather than creating a parallel harness.

## Acceptance criteria

- [ ] Adds realistic bugfix and refactor fixtures in TypeScript or Python with public-surface deterministic verifiers.
- [ ] Each case declares the case-relevant targeted skills and compares no skills, targeted skills, and all marketplace skills under the pilot containment boundary.
- [ ] Scoring covers behavior, regression tests, strict type/lint/format gates, diff scope, safety, and a calibrated semantic architecture rubric only where deterministic checks are insufficient.
- [ ] Sample counts and cross-case aggregation are declared before provider execution, and artifacts remain compatible with the pilot result schema.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
