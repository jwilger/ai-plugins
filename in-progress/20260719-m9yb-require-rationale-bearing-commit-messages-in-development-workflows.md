---
title: Require rationale-bearing commit messages in development workflows
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Add development-discipline guardrails that require every authored commit to include a concise Conventional Commit subject and a non-empty body explaining the motivation, tradeoff, or failure being prevented—not merely what files changed.

## Context / Why

## Acceptance criteria

- [x] Applicable commit guidance explicitly requires a message body that explains why the change is necessary.
- [x] Behavior tests reject or flag subject-only commits in workflows governed by the plugin.
- [x] Documentation gives a concise compliant commit example and preserves the no-Co-Authored-By rule.

## Subtasks

## Notes / Log

- 2026-07-19: Completed in commits 04d4326, b1b0b04, and 5eb1761. Provider-backed behavior eval passed 6/6 across Claude and Codex full-marketplace providers; plugin-eval analysis scored 100/100 (grade A); focused coverage tests passed 20/20; final local just ci passed all gates including 578 Bats tests and mutation testing; fresh-context final diff review was clean; exact GitHub Actions run 29706275650 passed for 5eb1761f7a6a89f6d0fad94c47017d06fbfc4d30.
