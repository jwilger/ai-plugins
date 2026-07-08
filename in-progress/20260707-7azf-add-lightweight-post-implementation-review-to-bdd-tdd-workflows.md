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
- 2026-07-07: Manual CodeRabbit full review completed on PR #41 and CodeRabbit status is success. Because the PR is stacked on development-discipline-final-review rather than main, GitHub Actions did not run normal main-branch CI; local validation evidence is recorded in the PR body. PR is clean and waits on #36 to merge before retarget/revalidation.
- 2026-07-07: After PR #36 merged, merged origin/main into development-discipline-tdd-light-review, resolved development-discipline version/test-list conflicts to keep TDD lightweight-review changes at 0.3.0, reran development-discipline plugin test, focused dry-run eval, marketplace validation, plugin-eval analysis 100/100, and pushed merge commit 0ab7e98.
- 2026-07-07: After PR #40 merged, merged origin/main containing the full Tiber chain into development-discipline-tdd-light-review, resolved README catalog to development-discipline 0.3.0 and Tiber 0.4.0, reran development-discipline plugin test, focused dry-run eval, marketplace validation, plugin-eval analysis 100/100, and pushed merge commit dc0e879.
- 2026-07-08: PR #41 is green and mergeable after updating onto main; CI gate passed on run 28907092317. Current blocker is required fresh non-self approval because the previous jwilger-ai-bot approval was dismissed by the branch update.
