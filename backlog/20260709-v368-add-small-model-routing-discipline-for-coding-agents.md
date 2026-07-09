---
title: Add small-model routing discipline for coding agents
blocked_by: []
blocks: []
tags: [development-discipline, model-routing, codex, claude-code, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add marketplace guidance and helper-agent behavior so Codex and Claude Code can route bounded, low-risk coding-agent work to small fast models while escalating before quality risk matters.

## Context / Why

Put the behavior in development-discipline, not agentic-systems-engineering. The ticket should not pin the implementation to specific plugin version bump numbers; the implementation should bump versions according to marketplace conventions. Codex routing should use the exact small Codex model name rather than the shorthand mentioned in planning, and Claude Code routing should use the Haiku alias. Do not treat Claude /fast as a cost-saving mechanism. Do not change global/default models, and do not downgrade advisor work, final review, security review, ambiguous debugging, architecture, destructive changes, or completion claims.

## Acceptance criteria

- [ ] development-discipline includes a model-routing skill that defines conservative cheap/default/strong routing criteria for coding-agent work.
- [ ] Codex and Claude Code get fast read-only helper-agent guidance, or the implementation documents why current harness support cannot do this yet.
- [ ] Existing development workflow skills cross-reference model-routing where small-model delegation decisions naturally arise.

## Subtasks

## Notes / Log
