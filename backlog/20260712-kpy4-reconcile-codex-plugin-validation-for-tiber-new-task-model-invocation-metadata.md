---
title: Reconcile Codex plugin validation for Tiber new-task model invocation metadata
blocked_by: []
blocks: []
tags: [tiber, codex, validation, developer-experience, minor-review-finding]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Resolve the pre-existing mismatch between Tiber's new-task skill metadata and the generic Codex plugin validator, which currently rejects disable-model-invocation.

## Context / Why

Discovered while validating the unrelated Clap parser release slice. The field predates that change and may be intentional for Claude Code, so remediation must preserve multi-harness behavior instead of changing the active parser ticket.

## Acceptance criteria

- [ ] The intended Claude Code and Codex semantics of new-task model invocation metadata are documented from authoritative harness behavior.
- [ ] Tiber validates for Codex without breaking the intentional Claude Code new-task behavior or duplicating harness-specific skill content unnecessarily.
- [ ] The generic plugin validator and the repository's canonical manifest/plugin gates pass for Tiber.

## Subtasks

## Notes / Log
