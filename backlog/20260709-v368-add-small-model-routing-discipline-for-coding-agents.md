---
title: Add small-model routing discipline for coding agents
blocked_by: []
blocks: []
tags: [development-discipline, model-routing, codex, claude-code, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add conservative model-routing guidance for sending bounded, low-risk helper work to small fast models while escalating before ambiguity or quality risk matters.

## Context / Why

Use gpt-5.3-codex-spark for eligible Codex helper work and the haiku alias for eligible Claude Code helper work, after verifying current harness availability at implementation time. These are task-local helper routes, not global/default model changes. Eligible work is bounded, read-only or easily reversible, and independently verifiable. Advisor work, final review, security review, architecture, ambiguous debugging, destructive changes, and completion claims must not be downgraded; Claude /fast is not a cost-routing substitute.

## Acceptance criteria

- [ ] development-discipline includes a model-routing skill that defines conservative cheap/default/strong routing criteria for coding-agent work.
- [ ] Codex and Claude Code get fast read-only helper-agent guidance, or the implementation documents why current harness support cannot do this yet.
- [ ] Existing development workflow skills cross-reference model-routing where small-model delegation decisions naturally arise.
- [ ] Behavior fixtures cover accepted cheap-model delegation, rejected cheap-model delegation, and rejection of Claude /fast as a cost-saving substitute.
- [ ] Marketplace docs and metadata are updated without hard-coding this ticket to a specific version bump number.
- [ ] Validation includes JSON checks, formatting, relevant coverage checks, eval dry-run, and focused provider-backed evidence where credentials allow.
- [ ] The routing matrix names gpt-5.3-codex-spark and the Claude haiku alias after current availability verification, defines harness fallbacks, and mandates escalation for every excluded high-risk category.

## Subtasks

## Notes / Log
