---
title: Reject empty skills-only Codex eval plugin selections
blocked_by: []
blocks: []
tags: [evals, codex, validation, plugin-loading, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make an explicitly supplied empty --plugins selection fail for skills-only-marketplace Codex eval homes while preserving the omitted-list full-marketplace behavior.

## Context / Why

Deferred caused MINOR from the lightweight review of 20260709-spx8. The focused benchmark is protected because cases.cjs rejects an empty standard plugin union, but prepare-codex-home.mjs currently parses --plugin-mode skills-only-marketplace --plugins "" as an empty array and successfully prepares a zero-plugin home. The direct preparation API should distinguish an omitted allowlist from an explicitly empty one.

## Acceptance criteria

- [ ] skills-only-marketplace with an explicitly supplied empty --plugins value fails with a clear validation error.
- [ ] Unknown skills-only plugin names fail before replacing or writing the eval home.

## Subtasks

## Notes / Log
