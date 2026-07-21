---
title: Stop irrelevant review findings from creating follow-up work
blocked_by: []
blocks: []
tags: [development-discipline, final-review, risk-scout, relevance, mcp, correctness, major, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Make findings from the final-review risk scout pass the same relevance checks as other review findings. Unsupported or out-of-scope observations must not block delivery or create unnecessary backlog tickets.

## Context / Why

Implementation notes: A caused MAJOR correctness finding from 20260713-rygd showed that scout findings bypass the normal relevance gate. A shallow scout can label an out-of-scope or generic observation as acceptance_criteria and force follow-up-ticket requirements because scout findings lack matched_context or changed_diff_evidence and are inserted directly. The mandatory scout must not have a weaker evidence contract than the lenses it plans.

## Acceptance criteria

- [x] Initial and delta risk-scout findings pass through the same deterministic relevance validation and filtering used for normal lens findings before persistence, disposition, blocker calculation, verifier selection, or follow-up-ticket requirements.
- [ ] Scout findings claiming acceptance_criteria, user_request, or explicit_user_concern relevance provide exact matched_context; cross-cutting findings provide in-scope changed_diff_evidence; prior-defense findings provide the defense ID and new contradictory changed-diff evidence.
- [ ] A missing, mismatched, generic, or out-of-scope relevance claim is rejected or retained only as a non-actionable report and cannot force a backlog ticket, verifier, blocker, or review reset.
- [ ] Valid scout findings retain their evidence through authoritative state, out-of-scope reports, later assignments, and disposition without bypassing normal policy.
- [ ] Focused Rust and public JSON-RPC tests cover a shallow scout falsely labeling a generic suggestion as an acceptance criterion, mismatched context, missing changed-diff evidence, valid exact context, and equivalent initial and delta behavior.

## Subtasks

## Notes / Log

- 2026-07-14: Consolidation check found no duplicate. Priority evidence: high recurring workflow value, medium impact, medium-high likelihood for shallow LLM observations, and moderate localized implementation cost.
