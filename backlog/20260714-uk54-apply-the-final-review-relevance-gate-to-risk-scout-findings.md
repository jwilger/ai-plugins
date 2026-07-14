---
title: Apply the final-review relevance gate to risk-scout findings
blocked_by: []
blocks: []
tags: [development-discipline, final-review, correctness, relevance, major]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Route initial and delta risk-scout findings through the same deterministic relevance gate as normal lens findings so generic observations cannot pollute or block the backlog.

## Context / Why

Final-review correctness finding from 20260713-rygd. This is not covered by 20260714-jyu9, which supplies conditional-lens objectives but does not enforce relevance evidence; 20260714-24xa covers split/budget contracts; 20260714-ra58 strengthens prose-fixture semantics. This ticket owns the production relevance gate for all initial and delta scout findings.

## Acceptance criteria

- [ ] Initial and delta risk-scout findings pass through the same deterministic relevance validation and filtering used for normal lens findings before persistence, disposition, blocker calculation, or follow-up-ticket requirements.
- [ ] Scout findings claiming acceptance_criteria, user_request, or explicit_user_concern relevance must provide exact matched_context; cross-cutting findings must provide in-scope changed_diff_evidence; prior-defense findings must provide the defense ID and new contradictory changed-diff evidence.
- [ ] A missing, mismatched, generic, or out-of-scope relevance claim is rejected or retained only as a non-actionable report and cannot force a backlog ticket, verifier, blocker, or review reset.
- [ ] Valid scout findings retain their evidence through authoritative state, out-of-scope reports, and later lens assignments without bypassing normal disposition rules.
- [ ] Focused Rust and public JSON-RPC tests cover a shallow scout falsely labeling a generic suggestion as an acceptance criterion, mismatched context, missing changed-diff evidence, a valid exact-context finding, and equivalent initial/delta behavior.

## Subtasks

## Notes / Log

- 2026-07-14: Consolidation check found no duplicate. Priority evidence: high recurring workflow value, medium impact, medium-high likelihood for shallow LLM observations, and moderate localized implementation cost.
