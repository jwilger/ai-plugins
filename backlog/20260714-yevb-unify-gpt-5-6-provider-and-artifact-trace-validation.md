---
title: Use one consistent check for GPT-5.6 execution records
blocked_by: []
blocks: []
tags: [minor, evals, architecture, trace, maintainability]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Replace two separate implementations of GPT-5.6 execution-record validation with one shared check. This reduces the chance that the live provider and the post-run checker disagree about whether an evaluation is valid.

## Context / Why

Implementation notes:\n\nVerified MINOR architecture finding from 20260709-spx8: both enforcement boundaries independently repeat item, notification, raw-response-item, and server-request validation while trace-policy.mjs shares only allowlists/helpers, leaving lifecycle rules vulnerable to drift. A later lightweight review of the successful-turn fix found additional caused MINOR gaps that belong in the same complete-validator work: lifecycle start/completion currently accept the error-notification turnId fallback instead of requiring params.turn.id; completion is not proven terminal relative to later allowed notifications; turn-scoped notifications are not uniformly checked for matching active thread/turn identifiers; and the shared helper's negative-path parity matrix does not cover all schema, ordering, duplication, and retryable-error branches.

## Acceptance criteria

- [ ] One pure shared validator owns the complete trace decision and structured rejection reason, and both trace-enforced-codex-provider.mjs and check-gpt56-execution-isolation.mjs call it with parity tests.
- [ ] The shared validator enforces method-specific Codex 0.144.3 notification shapes: lifecycle start/completion require nonblank params.threadId and params.turn.id, while error notifications use their protocol-defined identifiers.
- [ ] The shared validator proves completion is terminal and every relevant turn-scoped notification is ordered within, and associated with, the single active thread/turn lifecycle.
- [ ] Provider/checker parity tests cover missing and duplicate start/completion, completion-before-start, malformed and mismatched identifiers, post-completion events, and retryable errors outside or mismatched to the active turn.

## Subtasks

## Notes / Log

- 2026-07-14: 2026-07-14 formal final-review pass 1 for 20260709-spx8 reconfirmed missing provider/checker parity cases for retryable errors before/after the active turn, mismatched thread/turn identifiers, and duplicate turn/started. These are covered by the existing comprehensive lifecycle parity criterion; deferred as MINOR without changing the frozen diff.
