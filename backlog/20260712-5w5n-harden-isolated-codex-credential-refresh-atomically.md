---
title: Harden isolated Codex credential refresh atomically
blocked_by: []
blocks: []
tags: [evals, codex, credentials, hardening, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make isolated Codex credential refresh atomic and symlink-safe, and cover both supported credential filenames and file modes.

## Context / Why

Deferred MINOR findings from lightweight review of 20260712-kwbg. The current refresh copies directly over the destination and chmods afterward, so interruption or a filesystem error could leave a partial credential file, and an existing destination symlink could redirect the write outside the isolated eval home. Current focused coverage asserts refreshed auth.json contents but does not exercise .credentials.json or mode 0600.

## Acceptance criteria

- [ ] Credential refresh writes a temporary file in the target directory, sets mode 0600, and atomically renames it over a regular destination without following destination symlinks outside the eval home.
- [ ] Focused tests cover auth.json and .credentials.json contents and mode 0600 for both initial seeding and stale-credential refresh.
- [ ] An interrupted or failed refresh leaves either the complete old target or the complete new target, never mutates the source credentials, and never logs credential contents.
- [ ] Focused tests explicitly clear ambient OPENAI_API_KEY for credential-refresh cases and prove that API-key runs neither seed nor replace auth.json or .credentials.json.

## Subtasks

## Notes / Log

- 2026-07-13: Formal review of 20260712-kwbg confirmed the existing source-immutability coverage gap and additionally found that the refresh regression inherits ambient OPENAI_API_KEY while no focused case proves the API-key no-copy branch. Added explicit API-key isolation/no-copy coverage to this existing MINOR hardening ticket rather than creating a duplicate.
