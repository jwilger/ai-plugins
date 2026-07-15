---
title: Prove the personal Codex plugin consumption path
blocked_by: []
blocks: []
tags: [codex, installation, quality, major, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Document and verify one exact, reproducible path for installing, updating, and smoke-testing this marketplace's quality-core plugins from a clean downstream repository in Codex.

## Context / Why

Codex is the primary personal-use target, but the current catalog does not prove a concrete end-to-end consumption workflow. Establish the smallest reliable path that makes engineering-standards, development-discipline, advisor, and relevant specialist skills discoverable in another project. Claude parity and broad public onboarding are secondary and must not expand this ticket.

## Acceptance criteria

- [x] From a clean disposable downstream repository, documented commands install the intended quality-core plugins through the current Codex marketplace mechanism.
- [x] A repeatable smoke check proves the expected plugins and representative skills are discoverable and usable in the downstream repository.
- [x] The install/update path is safe to rerun and detects stale, missing, or partially installed plugin state with actionable diagnostics.
- [x] The repository documents Codex as the primary path and clearly scopes Claude Code and general-user polish as secondary follow-up concerns.

## Subtasks

## Notes / Log

- 2026-07-15: Implemented through fd11e80: default and agentic Codex quality-core install/check workflow, strict marketplace/plugin/prompt schema validation, rerunnable stale/disabled repair, conflict/spoof/incompatibility diagnostics, downstream model-visibility smoke, and Codex-primary documentation. Verification: full just ci passed (388 Bats; 44 mutants, 38 caught/6 unviable), focused 28/28, real Codex 0.144.4 disposable install/check/removal smoke passed with downstream clean, and enforced final review completed 1/1 clean with no findings. Push CI run 29437294914 is in progress.
- 2026-07-15: Push CI is green for fd11e80: GitHub Actions run 29437294914 completed successfully with Eval config dry-run, Codex cross-harness manifests, and Quality gate all passing. https://github.com/jwilger/ai-plugins/actions/runs/29437294914
