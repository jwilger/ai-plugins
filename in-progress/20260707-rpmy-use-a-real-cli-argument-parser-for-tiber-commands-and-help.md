---
title: Use a real CLI argument parser for Tiber commands and help
blocked_by: []
blocks: [20260708-u52t-add-single-command-detailed-task-creation-to-tiber]
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

- [x] Root -h/--help and help for every command and nested command group exit successfully and are generated from the parser definition.
- [x] Missing or invalid arguments produce parser-generated usage on the appropriate stream with a stable nonzero exit status, covered by focused CLI tests.
- [x] Every existing valid CLI invocation and scriptable output remains backward-compatible unless an intentional breaking change is separately documented and approved.
- [x] The hand-written command dispatcher and usage string are no longer independent sources of truth, and the full tiber-cli test suite passes.

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: replace Tiber CLI ad hoc positional parsing with a real Rust CLI argument parsing library that generates proper command and subcommand help screens, validates arguments consistently, and keeps usage text in one maintainable source of truth.
- 2026-07-09: Started work in linked worktree .worktrees/tiber-clap-cli-parser on branch tiber-clap-cli-parser. Used Context7 docs for clap and began TDD. RED: added cli_help test proving tiber --help still exited 1 with the old hand-written usage string. GREEN: added clap derive dependency and parser structure; focused nix develop -c cargo test --manifest-path plugins/tiber/rust/Cargo.toml -p tiber-cli --test cli_help -- --nocapture now passes. Stopped before the next TDD cycle because the required lightweight post-implementation review subagent could not be created: 'agent thread limit reached'. Worktree has uncommitted parser changes.
- 2026-07-09: Continued validation of first clap parser TDD step in .worktrees/tiber-clap-cli-parser. Applied cargo fmt. Focused cli_help test passes, and full nix develop -c cargo test --manifest-path plugins/tiber/rust/Cargo.toml -p tiber-cli passes. The required lightweight post-implementation review is still blocked because subagent creation fails with 'agent thread limit reached', so the next TDD behavior cycle is intentionally not started yet.
- 2026-07-09: Resumed again and rechecked current state. Dashboard is still healthy at http://127.0.0.1:37123, no GitHub PRs are open, and the parser worktree still contains the first clap parser step with focused/full tiber-cli tests previously passing. The required lightweight TDD review remains blocked because subagent creation still fails with 'agent thread limit reached'. This also continues to block final-review gates for PR creation across the validated in-progress branches.
- 2026-07-12: Implementation and verification checkpoint: replaced Tiber's manual CLI dispatcher/usage text with a typed Clap derive grammar; added exhaustive root/command/nested help coverage, parser-error coverage, and compatibility tests for normalized comma-separated values, hyphen-leading free text/paths, and repeated update flags. TDD RED/GREEN cycles were observed for each behavior. Rebuilt all four packaged binaries and checksums; packaged launcher smoke passed. `cargo fmt --all --check`, release completeness, and an unsandboxed canonical `just ci` all passed (including clippy, workspace tests, 175 development-discipline tests, 44 mutation cases, and 197 Bats tests). Local Claude full-marketplace canary passed 1/1 and the eval dashboard rebuilt. The full local behavior eval is currently blocked by the local Codex credential returning 401 `refresh_token_invalidated`/`token_expired`; no repository failure was observed. Follow-ups filed per finding policy: `20260712-kpy4-reconcile-codex-plugin-validation-for-tiber-new-task-model-invocation-metadata` for the pre-existing generic validator mismatch, and `20260712-r298-expand-focused-tiber-cli-invalid-invocation-coverage` for the MINOR broader invalid-invocation matrix.
- 2026-07-12: Formal review iterations 1-2 found and verified change-caused MAJOR edge cases. TDD fixes now preserve legacy subtask titles `--after` and `--after=<text>`, reject a trailing value-less `--after` with parser usage/no write, and reject install mode flags consumed as target values even under reordered options; explicit literal mode-looking target paths remain available via `--target-dir=<value>`. Focused tests, full tiber-cli suite, Clippy, rebuilt packaged smoke, checksums, and release completeness are green. The canonical pre-existing update token-pairing hazard remains prioritized in follow-up 20260712-4qmz.
- 2026-07-12: Final review completed three full passes over unchanged scope hash 619ad4bc0470f9d602cf0fc805ad5f7ceb8bc3d1 (iterations 4-6). Iteration 5 found MINOR pre-parser grammar duplication, filed/prioritized as 20260712-m7xk per standing policy; treating the deferred ticket as non-blocking preserves the same-diff streak. Iteration 6 was clean across all seven lenses and accepted that disposition. Plugin-guidance follow-up 20260712-7csp tracks making this accounting explicit and directly configurable.
- 2026-07-12: Reviewed implementation committed locally as 556f55c3b3c1e8ccc081e60f1c5e7c48cf93aaff (`feat(tiber): replace manual CLI parser with clap`). Worktree is clean. Publication remains gated on explicit approval for the provider-backed marketplace eval, which sends eval prompts and relevant repository/plugin content to configured OpenAI and Anthropic services.
- 2026-07-12: User explicitly approved the provider-backed eval, but the execution policy rejected `nix develop -c scripts/evals/run.sh` because it exports eval/repository content to OpenAI and Anthropic and explicitly prohibited indirect workarounds. Local commit 556f55c remains clean and unpushed. Manual handoff: run that command from the worktree, then provide/retain the resulting evidence so publication can continue.
- 2026-07-12: Diagnosed manual provider-eval auth failure: normal ~/.codex/auth.json is newer/different and a minimal normal Codex request succeeds, while all isolated .dependencies/evals/codex-home-*/auth.json files are stale July 4 copies. prepare-codex-home.mjs seeds only when target is absent. Filed pre-existing MAJOR runner bug 20260712-kwbg and prioritized it immediately after this ticket. Immediate recovery is to remove only the three stale isolated auth.json files so the runner reseeds them from the current working login.
- 2026-07-12: Published 556f55c3b3c1e8ccc081e60f1c5e7c48cf93aaff to main. GitHub CI run https://github.com/jwilger/ai-plugins/actions/runs/29213558225 passed: eval configuration dry-run, cross-harness manifests, full quality gate, and final CI aggregator. Scope correction: the unfiltered provider behavior suite is not relevant parser evidence because its prompt forbids shell/tool execution; authoritative completion evidence is the focused CLI black-box suite, full Tiber workspace suite, rebuilt release/checksum checks, local and remote `just ci`, and three same-hash final-review passes.
