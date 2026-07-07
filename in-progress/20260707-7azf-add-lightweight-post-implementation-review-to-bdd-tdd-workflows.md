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
