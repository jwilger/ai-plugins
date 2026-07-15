---
title: Make the HOME-unset development-discipline launcher test provide trusted Bash
blocked_by: []
blocks: [20260715-yvha-make-development-discipline-release-parity-fixture-use-a-fixed-clock]
tags: [development-discipline, tests, nix, hermeticity, ci, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Keep the HOME-unset Cargo-fallback startup test focused on missing-Cargo behavior by providing the Nix devshell’s trusted Bash on its scrubbed PATH.

## Context / Why

The canonical `just ci` gate consistently fails `development-discipline MCP Cargo fallback handles unset HOME` with status 127 and `env: 'bash': No such file or directory`. The fixture invokes the Bash launcher under `env -i PATH=/bin:/usr/bin`; on NixOS neither path contains Bash, so the shebang fails before the launcher can exercise its intended HOME-unset/missing-Cargo branch. This failure is unchanged by and blocks final verification of the deterministic release-parity ticket.

## Acceptance criteria

- [x] The HOME-unset fixture starts the repository launcher with a trusted Bash while still omitting HOME and excluding Cargo from PATH.
- [x] The focused test reaches and asserts the launcher’s `development-discipline.mcp.missing_cargo` diagnostic instead of failing at shebang resolution.
- [x] The complete MCP startup Bats file and repository CI gate pass without weakening the launcher’s untrusted-tool protections.

## Subtasks

## Notes / Log

- 2026-07-15: Completed in commit 087fa9204e2627f8e9e4e52e3e62df17d1022ef0, pushed directly to main. The fixture resolves trusted Bash before `env -i`, keeps HOME/CARGO/CARGO_HOME and inherited PATH absent, and reaches `development-discipline.mcp.missing_cargo`. Verification: focused test green; all 32 MCP startup tests green; local `nix develop -c just ci` green (44 mutation cases, 344 Bats); formal low-risk final review clean with one correctness lens and no findings; trunk CI run https://github.com/jwilger/ai-plugins/actions/runs/29395768157 green including aggregate CI gate.
