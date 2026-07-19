---
title: Follow each repository’s own delivery workflow
blocked_by: []
blocks: []
tags: [codex, workflow, development-discipline, engineering-standards, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make shared development guidance follow the repository’s stated way of delivering changes—such as direct updates to the main branch, pull requests, or local-only work—while keeping the same safety and verification standards.

## Context / Why

Implementation notes:\n\nReusable skills currently conflict with repositories that deliver directly to main, including this one. Define policy precedence as repository instructions first, workflow router second, and specialist skills third. Support direct-to-trunk, PR/MR, and local-only workflows while preserving safety, fresh verification, and evidence-before-claims.

## Acceptance criteria

- [ ] Skill guidance explicitly gives repository-local instructions precedence over generic delivery defaults.
- [ ] The workflow supports direct-to-trunk, PR/MR, and local-only delivery without inventing a pull-request requirement.
- [ ] Commit, push, review, and CI actions remain proportional to risk and require authorization for destructive or externally visible operations.
- [ ] Behavior fixtures cover all supported delivery modes and reject contradictory specialist guidance after routing.
- [ ] Documentation explains the precedence chain and the evidence required before claiming delivery complete.

## Subtasks

## Notes / Log
