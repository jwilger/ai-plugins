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

- [ ] Preparing full-marketplace, targeted-plugins, and no-plugins Codex eval homes replaces stale seeded credentials with the current auth source while preserving mode 0600 and never logging credential contents.
- [ ] Focused tests reproduce a pre-existing stale isolated auth file and prove a subsequent preparation refreshes it from a newer/different source without mutating the source home.
- [ ] API-key-backed runs continue to avoid copying ChatGPT credentials, and credential-source/target isolation guards remain enforced.

## Subtasks

## Notes / Log

- 2026-07-12: Lightweight TDD review identified two MINOR hardening/coverage follow-ups: atomic and symlink-safe destination replacement, plus explicit coverage for both credential filenames and mode 0600. Per the project disposition policy these are deferred without changing the current diff or resetting review progress. Tracked as 20260712-5w5n-harden-isolated-codex-credential-refresh-atomically, prioritized behind the existing MAJOR eval/Tiber defects.
