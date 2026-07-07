---
title: Add lightweight post-implementation review to BDD/TDD workflows
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

In the development-discipline plugin, update BDD/TDD practice guidance so that after each implementation step, the agent runs a less intensive review before moving to the next test cycle. This review should use the same general review lenses as the final-review skill, but combine them into a single fresh-context review subagent and require only one clean review before continuing.

## Context / Why

## Acceptance criteria

- [ ] The behavior is covered by development-discipline eval cases that exercise the lightweight post-implementation review loop.
- [ ] The development-discipline BDD/TDD guidance runs a lightweight review after each implementation step and before moving to the next test cycle.
- [ ] The lightweight review uses the same general repository-agnostic review lenses as final-review, but combines them into one fresh-context review subagent per implementation step.
- [ ] The lightweight review requires one clean review before the agent continues to the next red/green/refactor testing cycle, or records and addresses/defends any findings before continuing.

## Subtasks

## Notes / Log

- 2026-07-07: In progress on branch development-discipline-tdd-light-review. Local signed commit e7fe1d1 created with focused validation; stacked on PR #36 and waiting for #36 to merge before rebasing/opening PR.
- 2026-07-07: Opened stacked PR #41 against development-discipline-final-review: https://github.com/jwilger/ai-plugins/pull/41. Branch rebased onto current PR #36 head d92b75f; validation passed with development-discipline plugin bats test, focused dry-run eval case, marketplace validation, plugin-eval analysis 100/100, and git diff --check.
