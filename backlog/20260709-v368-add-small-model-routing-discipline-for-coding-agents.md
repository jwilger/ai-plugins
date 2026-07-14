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

- [ ] development-discipline includes a model-routing skill that defines conservative cheap/default/strong routing criteria for coding-agent work.
- [ ] Codex and Claude Code get fast read-only helper-agent guidance, or the implementation documents why current harness support cannot do this yet.
- [ ] Existing development workflow skills cross-reference model-routing where small-model delegation decisions naturally arise.
- [ ] Behavior fixtures cover accepted cheap-model delegation, rejected cheap-model delegation, and rejection of Claude /fast as a cost-saving substitute.
- [ ] Marketplace docs and metadata are updated without hard-coding this ticket to a specific version bump number.

## Subtasks

## Notes / Log
