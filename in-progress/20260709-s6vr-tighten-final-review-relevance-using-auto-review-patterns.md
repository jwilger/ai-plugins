---
title: Tighten final-review relevance using auto_review patterns
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Tighten the development-discipline final-review skill so subagent findings stay focused on the active change surface and the user's stated task, drawing useful relevance/filtering patterns from ../auto_review while preserving the existing review lenses and three-consecutive-clean rule.

## Context / Why

Recent final-review cycles produced findings outside the active ticket's intended surface. The skill should keep broad lenses, but require reviewers and the main agent to connect each finding to the requested change, PR scope, acceptance criteria, or explicit user concern before treating it as actionable.

## Acceptance criteria

- [ ] Final-review skill preserves the existing review lenses and three-consecutive-clean rule.
- [ ] Behavior/eval cases cover the relevance-filtering behavior and defense/out-of-scope handling.
- [ ] Skill instructs subagents to classify findings as actionable only when tied to the active task, PR/diff scope, acceptance criteria, explicit user concern, or a real cross-cutting safety/release risk.
- [ ] Implementation reviews ../auto_review for relevant review-focus patterns and incorporates applicable ideas without coupling this plugin to that project.
- [ ] This ticket is reviewed by dogfooding the development-discipline final-review runner rather than using the old manual final-review process.
- [ ] Project-local TOML and explicit plan arguments resolve pre_filter, lens_review, post_filter, and verifier model roles with precedence explicit args, project config, harness defaults, then generic roles.
- [ ] Codex harness defaults account for gpt-5.6-luna, gpt-5.6-sol, and gpt-5.6-terra while remaining overridable by project TOML and explicit plan arguments.
- [ ] Tests and behavior eval cases cover model-routing configuration and precedence without allowing the MCP server to spawn review agents.

## Subtasks

## Notes / Log
