---
title: Make Tiber opaque to host repositories and agents
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

- 2026-07-09: Validated and pushed signed branch origin/tiber-opaque-host-repos at 4c030a6. PR creation is waiting on final-review fresh-subagent capacity; subagent spawn currently fails with thread limit reached.
- 2026-07-09: Revalidated branch origin/tiber-opaque-host-repos at 4c030a6 from the linked worktree. Full nix develop -c just ci passed after cleaning stale shared Cargo artifacts for tiber-git; node scripts/evals/build-site.mjs wrote site/evals/index.html with no tracked diff; plugin-eval static analysis for plugins/tiber/skills/tiber and plugins/tiber/skills/new-task both scored 100/100 with only informational coverage-artifact notes. PR creation is still blocked by the required final-review fresh-subagent gate failing with 'agent thread limit reached'. Provider-backed just evals was attempted but rejected by the approval reviewer because it would export repository/plugin content to external eval providers; explicit user approval is needed before running it.
- 2026-07-09: Final-review iteration 1 found a valid correctness issue: dashboard reads in repos with origin could fail with tiber_lock_busy while another Tiber write held the lock. Fixed in 3b54e9d by falling back to a local task snapshot on lock-busy, adding an origin+lock dashboard regression test, making release artifact replacement atomic to avoid Text file busy, rebuilding Tiber release artifacts, and rerunning full nix develop -c just ci successfully. Branch origin/tiber-opaque-host-repos is pushed at 3b54e9d; final-review clean loop restarted from that commit.
- 2026-07-09: Final-review follow-up fixed two valid findings: dashboard SSE reload suppression now parses JSON error payloads instead of substring-matching task content, and the unused remote sync success timestamp was removed. Validation: focused tiber-server tests passed; Tiber release artifacts rebuilt; full nix develop -c just ci passed. Pushed af7904a to tiber-opaque-host-repos. Final-review clean loop restarted from af7904a.
- 2026-07-09: Final-review iteration 1 follow-up fixed additional valid findings: SSE initial error events now count as seen so recovery success reloads; partial-create markers are scoped to the exact local tasks ref and stale markers no longer permit missing remote recreation; sync errors retain redacted stderr plus a scrubbed category; oversized task-blob behavior coverage was added to shared fixtures and Tiber/new-task benchmarks. Validation: focused create_list, mcp_stdio, and tiber-server tests passed; Tiber release artifacts rebuilt; full nix develop -c just ci passed. Pushed a697aed. Final-review clean loop restarted from a697aed.
- 2026-07-09: Final-review instruction-quality finding fixed: task_blob_too_large diagnostics no longer suggest conflict tools, and behavior fixture plus Tiber/new-task benchmarks now require omitting conflict tools for size-limit diagnostics. Updated stale MCP assertion, rebuilt Tiber release artifacts, reran focused mcp_stdio and full nix develop -c just ci successfully. Pushed b318d81. Final-review clean loop restarted from b318d81.
- 2026-07-09: Final-review follow-up fixed dashboard SSE error visibility and stale oversized-blob guidance: browser now shows an aria-live sync status for sanitized SSE error events and clears it on successful events; Tiber/new-task skills and README no longer route task_blob_too_large through conflict tools. Validation: scripts/evals/run.sh --dry-run passed; focused tiber-server tests passed; Tiber release artifacts rebuilt; full nix develop -c just ci passed. Pushed 193eabd. Final-review clean loop restarted from 193eabd.
- 2026-07-09: Final-review follow-up fixed additional valid findings: local deletion vs unchanged remote now preserves/publishes deletion and local deletion vs changed remote conflicts; task push uses the same bounded remote I/O timeout as fetch; dashboard Markdown rejects scheme-relative links and parent traversal before doc existence checks; SSE sync status stays in the accessibility tree and uses higher-contrast text. Validation: sync test binary passed, tiber-server tests passed, scripts/evals/run.sh --dry-run passed, release artifacts rebuilt, full nix develop -c just ci passed. Pushed 1626dbb. Final-review clean loop restarted from 1626dbb.
- 2026-07-09: Final-review follow-up fixed remaining valid findings: timed remote git commands now terminate the whole Unix process group so SSH/helper descendants cannot keep pipes open after timeout; initial dashboard loads fall back to local task snapshots for remote sync failures and render a redacted Task sync delayed status; the sync status live region remains mounted for assistive tech. Validation: git diff --check passed, scripts/evals/run.sh --dry-run passed, focused tiber-server and tiber-cli sync tests passed, Tiber release artifacts rebuilt, and full nix develop -c just ci passed including 11/11 mutants and 148 Bats tests. Pushed c14240c; final-review clean loop restarted from c14240c.
