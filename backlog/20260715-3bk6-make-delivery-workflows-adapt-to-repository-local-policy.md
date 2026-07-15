---
title: Make delivery workflows adapt to repository-local policy
blocked_by: []
blocks: []
tags: [codex, workflow, development-discipline, engineering-standards, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make development-discipline and engineering-standards follow the active repository's delivery policy instead of universally assuming pull requests or incremental remote pushes.

## Context / Why

Reusable skills currently conflict with repositories that deliver directly to main, including this one. Define policy precedence as repository instructions first, workflow router second, and specialist skills third. Support direct-to-trunk, PR/MR, and local-only workflows while preserving safety, fresh verification, and evidence-before-claims.

## Acceptance criteria

- [ ] Skill guidance explicitly gives repository-local instructions precedence over generic delivery defaults.
- [ ] The workflow supports direct-to-trunk, PR/MR, and local-only delivery without inventing a pull-request requirement.
- [ ] Commit, push, review, and CI actions remain proportional to risk and require authorization for destructive or externally visible operations.

## Subtasks

## Notes / Log
