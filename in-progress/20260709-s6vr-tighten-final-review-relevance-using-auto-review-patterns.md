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

## Subtasks

## Notes / Log
