---
title: Prove realpath-aware bidirectional GPT-5.6 workspace overlap rejection
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, tests, filesystem, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Extend workspace-isolation tests across ancestor, descendant, symlink-alias, explicit-auth, and default ~/.codex overlap cases.

## Context / Why

The current overlap matrix checks exact lexical equality only. The implementation uses realpath-aware bidirectional containment, but regressions to equality-only checks or loss of default-home protection would pass. This MINOR review finding was deferred from 20260709-spx8.

## Acceptance criteria

- [ ] Tests fail if overlap protection loses ancestor, descendant, symlink-alias, or default ~/.codex coverage.

## Subtasks

## Notes / Log

- 2026-07-14: Superseded by 20260713-2rd3, which now carries the full realpath-aware bidirectional overlap matrix.
