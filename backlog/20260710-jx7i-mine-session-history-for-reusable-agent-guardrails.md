---
title: Mine session history for reusable agent guardrails
blocked_by: []
blocks: []
tags: [skills, hindsight, guardrails, privacy, workflow]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a privacy-safe skill that mines available session history for recurring mistakes or user corrections, proposes durable guardrails, and routes each proposal to the correct project or reusable marketplace surface.

## Context / Why

Use targeted Hindsight recall/reflect first when available, otherwise the current session or history explicitly supplied by the user. Never crawl arbitrary home-directory transcripts. Candidate guardrails must be evidence-linked, scrubbed of secrets and private data, and classified as project rule, reusable plugin guidance, eval case, or no action. Compose with eval-case-reporter, development-discipline writing-skills, cross-project preflight, and worktree safety instead of reimplementing them. Show a sanitized preview and require explicit approval before any write, issue, or cross-repository mutation.

## Acceptance criteria

- [ ] Skill identifies candidate guardrails from session history without treating transient chatter as durable policy.
- [ ] Skill recommends whether each guardrail belongs in a reusable ai-plugins plugin or is project-specific.
- [ ] When destination is ambiguous, skill presents the recommendation and obtains user confirmation before writing.
- [ ] Skill includes a safe workflow for locating and changing ai-plugins when invoked from another project.
- [ ] History retrieval uses targeted Hindsight when available and otherwise only the current session or user-supplied history; it never performs broad home-directory transcript scraping.
- [ ] Every candidate is evidence-linked, scrubbed/anonymized, and classified as project rule, reusable plugin guidance, eval case, or no action with a reason.
- [ ] The skill composes with eval-case-reporter, writing-skills, preflight, and worktree safety rather than duplicating their write or reporting workflows.
- [ ] A sanitized exact preview and explicit user approval are required before any project, issue-tracker, or cross-repository write.

## Subtasks

## Notes / Log
