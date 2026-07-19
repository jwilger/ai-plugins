---
title: Make Tiber’s new-task settings valid in both coding tools
blocked_by: []
blocks: []
tags: [tiber, codex, validation, developer-experience, minor-review-finding]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Resolve a validation mismatch in Tiber’s new-task skill while preserving the intended behavior in both Codex and Claude Code. Each coding tool’s supported setting should be documented and validated without duplicating the skill unnecessarily.

## Context / Why

Implementation notes:\n\nDiscovered while validating the unrelated Clap parser release slice. The field predates that change and may be intentional for Claude Code, so remediation must preserve multi-harness behavior instead of changing the active parser ticket.

## Acceptance criteria

- [ ] The intended Claude Code and Codex semantics of new-task model invocation metadata are documented from authoritative harness behavior.
- [ ] Tiber validates for Codex without breaking the intentional Claude Code new-task behavior or duplicating harness-specific skill content unnecessarily.
- [ ] The generic plugin validator and the repository's canonical manifest/plugin gates pass for Tiber.

## Subtasks

## Notes / Log
