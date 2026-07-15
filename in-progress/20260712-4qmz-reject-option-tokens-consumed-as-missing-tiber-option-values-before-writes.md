---
title: Reject option tokens consumed as missing Tiber option values before writes
blocked_by: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
blocks: []
tags: [tiber, cli, major, safety, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Prevent malformed option sequences from being interpreted as literal values and reaching write-capable Tiber operations.

## Context / Why

Final review of the Clap migration confirmed this behavior already existed in the manual parser: update option pairs accepted the next recognized flag as a value, and `install-bin --target-dir --dry-run --apply` treated `--dry-run` as the directory while applying the install. This is a pre-existing MAJOR ordinary-mistake/data-or-filesystem mutation risk, not caused by ticket rpmy. Preserve legitimate hyphen-leading values through an unambiguous form such as `--field=--value` while rejecting missing values before writes.

## Acceptance criteria

- [x] When a value-taking option is immediately followed by a recognized option token, Tiber emits parser usage on stderr, exits with the stable parser error status, and performs no task or filesystem write.
- [x] Legitimate hyphen-leading option values remain expressible through a documented unambiguous syntax such as `--option=--value`.
- [x] Focused integration tests cover both update-task and install-bin no-write cases plus the supported explicit hyphen-leading value form.

## Subtasks

## Notes / Log

- 2026-07-12: Ticket rpmy's review-driven remediation now rejects `--dry-run`/`--apply` when supplied as separate `install-bin --target-dir` values (including reordered modes), while preserving explicit literal paths through `--target-dir=--dry-run` or `--target-dir=--apply`. Keep this backlog item focused on the remaining pre-existing update-field/recognized-option consumption cases and any generalized parser policy.
- 2026-07-15: Green update-parser increment committed and pushed as 6bad7e9; tiber-rust passes and lightweight review is clean. Further increments await a CI signal, but CI runs only for PRs while the final-review gate prohibits opening a PR before ticket completion.
- 2026-07-15: Maintainer confirmed this repository does not use PRs. Continue with local green gates on the feature worktree, run final review before merging to main, and verify trunk CI after push.
- 2026-07-15: Implementation reached the agreed safety boundary. Focused parser/help/compatibility suites, four-platform release rebuild and checksums, host release probes, full `just ci` (including mutation tests and 360 Bats tests), standard/canary eval wiring dry-runs, dashboard build, and scoped final review all passed. No final-review findings remain. Live provider-backed evals still require explicit user approval before direct-to-main delivery.
- 2026-07-15: 2026-07-15 delivery evidence: implementation and all acceptance criteria are complete at dfde335 (diff hash 8bd232ed0f510cea7d2c1c824d0ece1d1147747e). Focused parser suites, packaged aarch64 probes, full `nix develop -c just ci` (including 44 mutants and 360 Bats tests), behavior/canary dry-runs, dashboard build, and formal final review passed. A focused live Codex advisory eval produced 5/10 semantic passes for the pre-existing `tiber-new-task-command-backlog-capture` skill scenario; all 10 hard guards passed and the five misses concerned structured validation, not CLI parsing. The branch changes no skill or fixture text. Promptfoo also incorrectly expanded one intended sample into plugins × skills × coverage_kinds combinations; that measurement defect is recorded on 20260713-hgyz. Claude live evaluation was unavailable because no Claude authentication is configured. The approved Promptfoo share attempt was also unavailable because the CLI has no Promptfoo Cloud login. These external limitations do not weaken the deterministic no-write/parser evidence for this ticket.
