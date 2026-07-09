---
title: Use a real CLI argument parser for Tiber commands and help
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: replace Tiber CLI ad hoc positional parsing with a real Rust CLI argument parsing library that generates proper command and subcommand help screens, validates arguments consistently, and keeps usage text in one maintainable source of truth.
- 2026-07-09: Started work in linked worktree .worktrees/tiber-clap-cli-parser on branch tiber-clap-cli-parser. Used Context7 docs for clap and began TDD. RED: added cli_help test proving tiber --help still exited 1 with the old hand-written usage string. GREEN: added clap derive dependency and parser structure; focused nix develop -c cargo test --manifest-path plugins/tiber/rust/Cargo.toml -p tiber-cli --test cli_help -- --nocapture now passes. Stopped before the next TDD cycle because the required lightweight post-implementation review subagent could not be created: 'agent thread limit reached'. Worktree has uncommitted parser changes.
- 2026-07-09: Continued validation of first clap parser TDD step in .worktrees/tiber-clap-cli-parser. Applied cargo fmt. Focused cli_help test passes, and full nix develop -c cargo test --manifest-path plugins/tiber/rust/Cargo.toml -p tiber-cli passes. The required lightweight post-implementation review is still blocked because subagent creation fails with 'agent thread limit reached', so the next TDD behavior cycle is intentionally not started yet.
