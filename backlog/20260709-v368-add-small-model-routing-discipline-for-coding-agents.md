---
title: Add GPT-5.6 model-routing discipline for coding agents
blocked_by: []
blocks: []
tags: [development-discipline, model-routing, gpt-5.6, codex, claude-code, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add conservative task-local routing among GPT-5.6 Luna, Terra, and Sol for Codex coding-agent work, retain a current verified Claude equivalent where supported, and escalate before ambiguity, risk, or completion responsibility makes a cheaper route inappropriate.

## Context / Why

Use gpt-5.6-luna for bounded read-only or easily reversible helper work with independent verification, gpt-5.6-terra for normal substantive implementation and review, and gpt-5.6-sol for advisor work, blocking or disputed verification, architecture, security or human-safety analysis, ambiguous debugging, destructive changes, and completion claims. These are task-local helper routes rather than global defaults. Verify current harness support during implementation, fail visibly instead of silently downgrading, and keep Claude-specific routing aligned with its current supported aliases; Claude /fast is not a cost-routing substitute.

## Acceptance criteria

- [ ] The routing matrix names gpt-5.6-luna for bounded helpers, gpt-5.6-terra for normal substantive work, and gpt-5.6-sol for strong or escalated work, with explicit eligibility and exclusion rules.
- [ ] Implementation verifies the exact current harness model identifiers and availability; an unavailable requested route fails visibly or escalates rather than silently downgrading.
- [ ] Claude Code receives an equivalent current-harness helper and escalation policy where supported, or the implementation documents the missing capability; Claude /fast is never presented as a cost-routing substitute.
- [ ] Existing development workflow skills cross-reference model routing where delegation decisions naturally arise without duplicating the full matrix.
- [ ] Behavior fixtures cover accepted Luna delegation, Terra default work, required Sol escalation, ambiguous or high-risk work that cannot use Luna, and refusal to silently downgrade.

## Subtasks

## Notes / Log
