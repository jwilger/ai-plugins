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

## Subtasks

## Notes / Log
