---
title: Explain how to recover a reviewer slot safely
blocked_by: []
blocks: []
tags: [development-discipline, final-review, subagents, workflow, guidance, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Document how final review can recover when a completed reviewer still occupies a limited agent slot. The recovery must free the slot without reusing that reviewer or falsely claiming that the replacement review started with fresh context.

## Context / Why

Implementation notes: Observed during the Ctrl-C eval-runner final review: the collaboration runtime sometimes retained a completed reviewer and rejected the next fresh spawn at the global thread limit. The successful recovery was to trigger a no-work administrative follow-up on the retained completed agent, immediately interrupt that administrative turn, then spawn a brand-new fork_turns=none reviewer. Guidance must stress that the interrupted agent is not reused for review and that the new reviewer remains genuinely fresh. Document bounded retries and safe fallback behavior without fabricating attestations.

## Acceptance criteria

- [ ] Final-review guidance documents the administrative follow-up plus immediate interrupt sequence for releasing a retained completed-agent slot.
- [ ] The guidance requires the replacement reviewer to be a brand-new fork_turns=none subagent and explicitly forbids treating the interrupted administrative agent as fresh review context.
- [ ] Recovery instructions include bounded retry/reporting behavior when the runtime still refuses a fresh spawn, with tests or fixtures covering the guidance where applicable.

## Subtasks

## Notes / Log

- 2026-07-13: Confirmed recovery technique on 2026-07-13: when a completed retained subagent still consumes a concurrency slot, send it an administrative follow-up explicitly requesting no work and an immediate return, then call interrupt_agent after that return. This released the slot and allowed a fresh reviewer to spawn. Final-review guidance should document the sequence and require preserving the completed result before recycling.
- 2026-07-22: 2026-07-22 curation rejection: Lower value relative to cost than the retained cross-project final-review identity/restart blocker. This is readiness, fixture, scale, cleanup, or protocol-quality follow-up rather than the repeated root-cause delivery failure; no shadow ticket is retained.
