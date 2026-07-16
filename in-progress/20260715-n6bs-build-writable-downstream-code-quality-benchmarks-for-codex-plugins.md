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

Measure whether the marketplace makes Codex produce better code in disposable downstream repositories, comparing no plugins, targeted quality-core plugins, and the full marketplace.

## Context / Why

Current behavior evals mostly score read-only advice and do not establish implementation-quality lift. Build realistic writable feature, bugfix, and refactor scenarios with public-surface verifiers. Start with a real personal project when suitable; otherwise use a Rust CLI plus one TypeScript or Python service. Exact targeted-plugin composition from ticket hgyz is a prerequisite. Keep fixtures scrubbed and disposable.

## Acceptance criteria

- [ ] The benchmark runs representative writable feature, bugfix, and refactor scenarios in disposable downstream repositories.
- [ ] Each scenario compares no-plugin, targeted quality-core, and full-marketplace Codex conditions using the exact plugin set declared by the case.
- [ ] Scoring combines deterministic black-box tests, type and lint checks, mutation quality where appropriate, diff scope, and calibrated semantic rubrics only where deterministic checks are insufficient.
- [ ] Sample counts, aggregation rules, pass thresholds, and release gates are declared before execution and report both success rate and relevant reliability metrics.
- [ ] Artifacts preserve machine-readable results, reviewer-facing output, latency, token usage, and cost without exposing secrets or private project data.

## Subtasks

## Notes / Log
