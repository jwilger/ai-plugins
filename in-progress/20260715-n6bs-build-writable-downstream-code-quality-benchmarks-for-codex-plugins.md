---
title: Build writable downstream code-quality benchmarks for Codex plugins
blocked_by: [20260718-iksx-review-codex-subscription-auth-runtime-boundary-slice, 20260718-zcsh-review-benchmark-contract-and-scorer-slice]
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

Re-scoped after the first Promptfoo wiring security review. The immediate decision-quality need is directional evidence about marketplace-skill-driven code quality, not broad multi-language or full executable-plugin coverage. Run the expense-report feature case with three samples per condition using the Codex-bundled system-skill baseline plus no marketplace skills, targeted skills-only projections, and all-marketplace skills-only projections; use bounded execution, existing ChatGPT-backed Codex authentication copied into disposable per-sample homes, trusted source rebuilds, explicit failure taxonomy, and sanitized artifacts. Defer TypeScript bugfix/refactor expansion and true full-plugin runtime execution to follow-up tickets.

## Acceptance criteria

- [ ] Trusted post-turn scoring rebuilds candidate source in the verifier sandbox and combines public black-box behavior, format, clippy, locked tests, candidate-regression replay against the baseline, diff scope, and safety checks.
- [ ] The contract predeclares three samples, success rate, pass@3 capability, pass^3 reliability, diagnostic thresholds, and a non-promotional claim; provider, operational, provenance, safety, and candidate failures remain distinct.
- [ ] Allowlisted artifacts preserve input and composition hashes, tool/model versions, sanitized diff evidence, skill activations, latency, token usage, and cost; raw transcripts stay private and ephemeral, are secret-scanned, and are never shared.
- [ ] The diagnostic runs the Rust expense-report feature case in fresh disposable repositories for three samples each of no marketplace skills (Codex-bundled system skills remain), the declared quality-core marketplace skills, and all marketplace skills.
- [ ] Only sanitized skills-only plugin projections are loaded; live execution reuses an existing ChatGPT-backed Codex login through refresh-preserving run-scoped disposable auth linked into otherwise-isolated per-sample CODEX_HOME directories while blocking candidate access to authentication, sibling and host reads/writes, command network access, and enforcing finite wall, CPU, memory, process, output, and workspace limits.

## Subtasks

## Notes / Log

- 2026-07-17: 2026-07-16: Hosted CI repair is green. Fresh canonical `run-code-quality-benchmark.sh --dry-run` verified exactly 3 conditions x 3 samples and diagnostic gates; `--runtime-preflight` verified the pinned Nix execution closure. No prior sanitized result or live benchmark process exists. Live execution has not started because `CODE_QUALITY_OPENAI_API_KEY` is absent locally, no provider-key GitHub Actions secret/reference is configured, and the fail-closed contract forbids using normal Codex login state. Keep the task in progress until the canonical provider-backed run produces and passes sanitized evidence.
- 2026-07-17: Credential audit expanded: no repository-, GitHub environment-, or Codespaces-scoped provider key is configured; local `CODE_QUALITY_OPENAI_API_KEY` remains absent and local 1Password CLI remains unauthenticated. The canonical live run is still awaiting a dedicated key or an authenticated `op://` reference.
- 2026-07-17: After three consecutive autonomous goal audits, the canonical provider run remains at an external credential impasse: no dedicated key or authenticated local 1Password reference is available. The goal is being marked blocked without closing or moving this ticket; resume it immediately when the credential becomes available.
- 2026-07-18: 2026-07-17: Maintainer revised the authentication contract: retain fresh isolated per-sample CODEX_HOME directories, but reuse the machine's existing ChatGPT-subscription Codex authentication instead of requiring a dedicated API key. The trusted runner must copy auth into disposable homes without exposing it to candidate commands or mutating the operator's source credentials.
- 2026-07-18: 2026-07-18: Subscription-auth revision implemented test-first. The runner validates the existing private ChatGPT auth file, atomically seeds independent disposable sample homes, mounts only neutral writable snapshots for Codex refresh, injects no API-key environment, and removes disposable auth before secret scanning. Full focused benchmark suite (36 tests) and boundary suite (52 tests) pass.
- 2026-07-18: 2026-07-18: Lightweight review found that per-invocation snapshots discarded rotated refresh tokens. The revision now uses shared run-scoped disposable auth across sequential turns, persists validated refresh updates, and includes both a two-turn continuity regression and a real installed-Codex sandbox probe proving model tools cannot read auth.
