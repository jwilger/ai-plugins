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

Build a safe, non-promotional writable Rust benchmark that measures whether Codex produces better downstream code with no marketplace skills (while retaining Codex-bundled system skills), the declared quality-core marketplace skills, and all marketplace skills.

## Context / Why

Re-scoped after the first Promptfoo wiring security review. The immediate decision-quality need is directional evidence about marketplace-skill-driven code quality, not broad multi-language or full executable-plugin coverage. Run the expense-report feature case with three samples per condition using the Codex-bundled system-skill baseline plus no marketplace skills, targeted skills-only projections, and all-marketplace skills-only projections; use bounded execution, API-key-only auth, trusted source rebuilds, explicit failure taxonomy, and sanitized artifacts. Defer TypeScript bugfix/refactor expansion and true full-plugin runtime execution to follow-up tickets.

## Acceptance criteria

- [ ] Only sanitized skills-only plugin projections are loaded; live execution fails closed without dedicated API-key authentication, blocked sibling and host reads/writes, blocked command network access, and finite wall, CPU, memory, process, output, and workspace limits.
- [ ] Trusted post-turn scoring rebuilds candidate source in the verifier sandbox and combines public black-box behavior, format, clippy, locked tests, candidate-regression replay against the baseline, diff scope, and safety checks.
- [ ] The contract predeclares three samples, success rate, pass@3 capability, pass^3 reliability, diagnostic thresholds, and a non-promotional claim; provider, operational, provenance, safety, and candidate failures remain distinct.
- [ ] Allowlisted artifacts preserve input and composition hashes, tool/model versions, sanitized diff evidence, skill activations, latency, token usage, and cost; raw transcripts stay private and ephemeral, are secret-scanned, and are never shared.
- [ ] The diagnostic runs the Rust expense-report feature case in fresh disposable repositories for three samples each of no marketplace skills (Codex-bundled system skills remain), the declared quality-core marketplace skills, and all marketplace skills.

## Subtasks

## Notes / Log
