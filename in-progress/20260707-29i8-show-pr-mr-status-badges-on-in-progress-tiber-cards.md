---
title: Show PR/MR status badges on in-progress Tiber cards
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: in-progress dashboard cards should show a color-coded badge for PR/MR state when a pull request or merge request exists for the task. The Tiber plugin/skill should also instruct agents as deterministically as possible to add PR/MR link/info to tasks and keep the task PR/MR status updated so the in-progress card badge stays accurate.
- 2026-07-07: In progress on branch tiber-pr-status-badges. Committed e98c93d with dashboard PR/MR badges, structured CLI/MCP update fields, skill guidance, tests, 0.4.0 metadata, and rebuilt release binaries. Validation passed: cargo test --workspace, just validate-marketplace, release complete check, binary marker check, plugin-eval analyze.
- 2026-07-07: Opened PR #40: https://github.com/jwilger/ai-plugins/pull/40. Note: this Tiber version metadata will need rebase/order handling with PR #38 and PR #39.
- 2026-07-07: Pushed follow-up commit 8aad946 on PR #40: refactored update_task through TaskUpdate to satisfy clippy, fixed empty PR/MR fields so they clear instead of becoming unknown, added regression coverage, reran cargo clippy, cargo test --workspace, release completeness check, binary marker check, just validate-marketplace, and git diff --check.
- 2026-07-07: PR #40 latest CI run 28903597413 is green after follow-up commit 8aad946, including Quality gate and CI gate. CodeRabbit status is success, but the full review comment was rate-limited; still needs human approval and possibly another full review after the rate window clears.
- 2026-07-07: PR #40 was approved but became DIRTY after #36 merged. Merged origin/main into tiber-pr-status-badges, resolved README catalog conflict to keep development-discipline 0.2.0 and Tiber 0.4.0, reran just validate-marketplace, and pushed merge commit 08b08a8.
- 2026-07-07: After PR #39 merged, PR #40 became DIRTY again. Merged origin/main containing #38/#39 into tiber-pr-status-badges, resolved Tiber metadata to 0.4.0, retained both the in-progress reminder and lock retry changes, rebuilt all release binaries, verified binary markers for pr_mr_status and TIBER_LOCK_RETRY_TIMEOUT_MS, reran cargo test --workspace, clippy, just validate-marketplace, release completeness, and pushed merge commit 46ecb6c.
