---
title: Apply the final-review relevance gate to risk-scout findings
blocked_by: []
blocks: []
tags: [development-discipline, final-review, risk-scout, relevance, mcp, correctness, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Route initial and delta risk-scout findings through the same structured relevance validation and filtering used for normal lens findings before they can affect disposition or backlog work.

## Context / Why

A caused MAJOR correctness finding from 20260713-rygd showed that scout findings bypass the normal relevance gate. A shallow scout can label an out-of-scope or generic observation as acceptance_criteria and force follow-up-ticket requirements because scout findings lack matched_context or changed_diff_evidence and are inserted directly. The mandatory scout must not have a weaker evidence contract than the lenses it plans.

## Acceptance criteria

- [ ] Initial and delta risk-scout findings pass through the same deterministic relevance validation and filtering used for normal lens findings before persistence, disposition, blocker calculation, verifier selection, or follow-up-ticket requirements.
- [ ] Scout findings claiming acceptance_criteria, user_request, or explicit_user_concern relevance provide exact matched_context; cross-cutting findings provide in-scope changed_diff_evidence; prior-defense findings provide the defense ID and new contradictory changed-diff evidence.
- [ ] A missing, mismatched, generic, or out-of-scope relevance claim is rejected or retained only as a non-actionable report and cannot force a backlog ticket, verifier, blocker, or review reset.
- [ ] Valid scout findings retain their evidence through authoritative state, out-of-scope reports, later assignments, and disposition without bypassing normal policy.

## Subtasks

## Notes / Log

- 2026-07-14: Consolidation check found no duplicate. Priority evidence: high recurring workflow value, medium impact, medium-high likelihood for shallow LLM observations, and moderate localized implementation cost.
