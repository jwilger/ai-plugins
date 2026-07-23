---
title: Require explicit user approval before amending commits
blocked_by: []
blocks: []
tags: [bug, development-discipline, git, safety]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Treat additive commits as the default repair workflow and require explicit case-by-case user authorization before amending any existing commit.

## Context / Why

A prior PR repair amended a published commit and created avoidable recovery work. Shared/default-branch history must never be rewritten as routine repair. On other branches, an amend is allowed only when the user explicitly authorizes that specific amend; PR feedback and follow-up fixes default to new commits and must not trigger a force-push merely to replace history.

## Acceptance criteria

- [x] Relevant commit/PR guidance states that amending any commit requires explicit user authorization.
- [x] Default repair and follow-up workflow uses additive commits and does not force-push solely to replace an amended commit.
- [x] Shared or default-branch history is never amended as routine repair; any permitted amend elsewhere requires explicit case-by-case user authorization.
- [x] Behavior fixtures cover an agent repairing review feedback by adding a commit and refusing to amend or force-push without the required authorization.

## Subtasks

## Notes / Log

- 2026-07-23: Delivered direct-to-main through additive commits ending at 62cdd03217dced57b172c608e5b41e53d51542fd. Fast verification passes (5 Bats fixture tests, 24-skill coverage, applicable formatting, diff hygiene); exact focused provider eval eval-9pe-2026-07-23T01:41:25 passed all plugin-enabled variants and configured thresholds, with only the intended Claude no-plugin ablation failing; secret scan clean; structured final review completed with one clean iteration on diff f0d6f375cb685b6ad8b982b7b1de129b84a151c4. Awaiting terminal green CI run 29972260638 before closure.
- 2026-07-23: Terminal CI evidence: GitHub Actions run 29972260638 completed successfully for exact pushed SHA 62cdd03217dced57b172c608e5b41e53d51542fd; Quality gate, cross-harness manifests, eval dry-run, and CI gate all succeeded.
