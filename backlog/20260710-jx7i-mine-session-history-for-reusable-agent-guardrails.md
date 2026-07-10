---
title: Mine session history for reusable agent guardrails
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a skill that mines session history for durable guardrails and recommends their proper home.

## Context / Why

The skill must inspect session history for recurring mistakes or user corrections, propose reusable guardrails, distinguish project-specific rules from marketplace/plugin-level guidance, and ask the user to choose the destination when the recommendation is not unambiguous. It must know how to make ai-plugins changes even when invoked from another project.

## Acceptance criteria

- [ ] Skill identifies candidate guardrails from session history without treating transient chatter as durable policy.
- [ ] Skill recommends whether each guardrail belongs in a reusable ai-plugins plugin or is project-specific.
- [ ] When destination is ambiguous, skill presents the recommendation and obtains user confirmation before writing.
- [ ] Skill includes a safe workflow for locating and changing ai-plugins when invoked from another project.

## Subtasks

## Notes / Log
