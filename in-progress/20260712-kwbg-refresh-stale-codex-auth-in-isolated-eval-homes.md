---
title: Refresh stale Codex auth in isolated eval homes
blocked_by: []
blocks: []
tags: [evals, codex, authentication, bug, major]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Ensure provider-backed eval homes refresh their seeded Codex credentials after the user signs in again, instead of retaining revoked refresh tokens indefinitely.

## Context / Why

`scripts/evals/prepare-codex-home.mjs::seedAuth` copies `auth.json` and `.credentials.json` only when the isolated target does not exist. In the Tiber Clap ticket, the normal `~/.codex/auth.json` (July 9, working) differed from all three isolated eval copies (July 4), and the provider run failed repeatedly with `refresh token was revoked`. The isolated homes must remain separate from the real Codex home, but credential seeding must safely converge to the current source without printing secret material. This is a pre-existing MAJOR release/eval-operability defect discovered incidentally.

## Acceptance criteria

- [x] Preparing full-marketplace, targeted-plugins, and no-plugins Codex eval homes replaces stale seeded credentials with the current auth source while preserving mode 0600 and never logging credential contents.
- [x] Focused tests reproduce a pre-existing stale isolated auth file and prove a subsequent preparation refreshes it from a newer/different source without mutating the source home.
- [x] API-key-backed runs continue to avoid copying ChatGPT credentials, and credential-source/target isolation guards remain enforced.

## Subtasks

## Notes / Log

- 2026-07-12: Lightweight TDD review identified two MINOR hardening/coverage follow-ups: atomic and symlink-safe destination replacement, plus explicit coverage for both credential filenames and mode 0600. Per the project disposition policy these are deferred without changing the current diff or resetting review progress. Tracked as 20260712-5w5n-harden-isolated-codex-credential-refresh-atomically, prioritized behind the existing MAJOR eval/Tiber defects.
- 2026-07-13: Completed in signed commit 630038c1269a4d3b6d24e1bfc5394b20cdad112c, pushed to main. Verification: focused eval-config Bats 15/15; full local just ci green (including 44-mutant run and 198 Bats tests); GitHub CI run https://github.com/jwilger/ai-plugins/actions/runs/29217483064 fully green. Final review ran three full passes over unchanged diff hash bd9cca00d7d07a58304587cabbbdf07653aa2492 with no CRITICAL/MAJOR findings or unresolved blockers. Confirmed MINOR gaps in atomic/symlink-safe replacement, independent source/mode/filename assertions, and API-key branch coverage are tracked in prioritized follow-up 20260712-5w5n per policy. The coordinator reached clean_streak=3 but its separate verified-clean counter incorrectly prevented complete=true for ticket-routed findings; reproduction recorded on 20260712-7csp.
