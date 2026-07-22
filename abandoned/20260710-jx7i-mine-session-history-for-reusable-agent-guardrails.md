---
title: Turn recurring session mistakes into reusable safeguards
blocked_by: [20260708-dsfg-add-cross-project-change-preflight-skill-to-development-discipline]
blocks: []
tags: [development-discipline, skills, hindsight, guardrails, privacy, workflow]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a workflow that reviews available session history for repeated mistakes or corrections, proposes durable safeguards, and routes each proposal to the right project or shared plugin. It must protect private information and require approval before making external or cross-project changes.

## Context / Why

Implement this as plugins/development-discipline/skills/mine-session-guardrails for both supported marketplace harnesses after prerequisite 20260708-dsfg delivers the cross-project preflight it must compose with. Use targeted Hindsight recall/reflect first when available, otherwise the current session or history explicitly supplied by the user, so Hindsight remains optional. Never crawl arbitrary home-directory transcripts. Candidate guardrails must be evidence-linked, scrubbed of secrets and private data, and classified as project rule, reusable plugin guidance, eval case, or no action. Compose with eval-case-reporter, development-discipline writing-skills, the prerequisite preflight, and worktree safety instead of reimplementing them. Show a sanitized preview and require explicit approval before any write, issue, or cross-repository mutation.

## Acceptance criteria

- [ ] Skill identifies candidate guardrails from session history without treating transient chatter as durable policy.
- [ ] Skill recommends whether each guardrail belongs in a reusable ai-plugins plugin or is project-specific.
- [ ] When destination is ambiguous, skill presents the recommendation and obtains user confirmation before writing.
- [ ] Skill includes a safe workflow for locating and changing ai-plugins when invoked from another project.
- [ ] History retrieval uses targeted Hindsight when available and otherwise only the current session or user-supplied history; it never performs broad home-directory transcript scraping.
- [ ] Every candidate is evidence-linked, scrubbed/anonymized, and classified as project rule, reusable plugin guidance, eval case, or no action with a reason.
- [ ] The skill composes with eval-case-reporter, writing-skills, preflight, and worktree safety rather than duplicating their write or reporting workflows.
- [ ] A sanitized exact preview and explicit user approval are required before any project, issue-tracker, or cross-repository write.
- [ ] The skill is named mine-session-guardrails under development-discipline, supports both Claude Code and Codex with optional Hindsight, and updates both manifests, plugin/root documentation, semver, and relevant full-marketplace eval coverage.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Broad or expensive initiative with weaker near-term value-to-cost than the retained concrete defects and security work. Discovery does not obligate retention, and no hidden queue is kept.
