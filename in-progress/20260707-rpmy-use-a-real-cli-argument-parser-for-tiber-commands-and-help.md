---
title: Use a real CLI argument parser for Tiber commands and help
blocked_by: []
blocks: []
tags: [bug, tiber, cli, developer-experience]
claim:
  host: unknown
  session: unknown
---

## Summary

Replace Tiber's hand-written positional dispatch and usage string with a real Rust CLI parser so standard help works, invalid invocations fail consistently, and the command grammar has one maintainable source of truth.

## Context / Why

The current CLI returns the hand-written usage error for standard help and duplicates parsing rules across positional branches. A linked worktree at .worktrees/tiber-clap-cli-parser already contains uncommitted Clap-based work and a focused cli_help test. Preserve that work, keep existing valid scriptable invocations compatible, and treat generated parser help as the authoritative command contract.

## Acceptance criteria

- [ ] Root -h/--help and help for every command and nested command group exit successfully and are generated from the parser definition.
- [ ] Missing or invalid arguments produce parser-generated usage on the appropriate stream with a stable nonzero exit status, covered by focused CLI tests.
- [ ] Every existing valid CLI invocation and scriptable output remains backward-compatible unless an intentional breaking change is separately documented and approved.
- [ ] The hand-written command dispatcher and usage string are no longer independent sources of truth, and the full tiber-cli test suite passes.

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: replace Tiber CLI ad hoc positional parsing with a real Rust CLI argument parsing library that generates proper command and subcommand help screens, validates arguments consistently, and keeps usage text in one maintainable source of truth.
- 2026-07-09: Started work in linked worktree .worktrees/tiber-clap-cli-parser on branch tiber-clap-cli-parser. Used Context7 docs for clap and began TDD. RED: added cli_help test proving tiber --help still exited 1 with the old hand-written usage string. GREEN: added clap derive dependency and parser structure; focused nix develop -c cargo test --manifest-path plugins/tiber/rust/Cargo.toml -p tiber-cli --test cli_help -- --nocapture now passes. Stopped before the next TDD cycle because the required lightweight post-implementation review subagent could not be created: 'agent thread limit reached'. Worktree has uncommitted parser changes.
- 2026-07-09: Continued validation of first clap parser TDD step in .worktrees/tiber-clap-cli-parser. Applied cargo fmt. Focused cli_help test passes, and full nix develop -c cargo test --manifest-path plugins/tiber/rust/Cargo.toml -p tiber-cli passes. The required lightweight post-implementation review is still blocked because subagent creation fails with 'agent thread limit reached', so the next TDD behavior cycle is intentionally not started yet.
- 2026-07-09: Resumed again and rechecked current state. Dashboard is still healthy at http://127.0.0.1:37123, no GitHub PRs are open, and the parser worktree still contains the first clap parser step with focused/full tiber-cli tests previously passing. The required lightweight TDD review remains blocked because subagent creation still fails with 'agent thread limit reached'. This also continues to block final-review gates for PR creation across the validated in-progress branches.
