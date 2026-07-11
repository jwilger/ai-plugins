---
title: Add CI setup skills with staged fast-fail gating
blocked_by: []
blocks: []
tags: [ci, skills, engineering-standards, workflow-design]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Create reusable skill guidance for designing and scaffolding efficient CI pipelines, with generic principles separated from tool-specific implementation details.

## Context / Why

Audit existing engineering-standards and development-discipline skills first. The generic guidance should model CI as dependency stages: independent checks within a stage run in parallel, but expensive or slow checks depend on a fast validation stage so they do not consume time or compute when cheap checks already fail. Add tool-specific skills or references only where systems such as GitHub Actions, GitLab CI, or Forgejo require materially different syntax or capabilities.

## Acceptance criteria

- [ ] Existing CI-related plugin and skill guidance is audited, and the new responsibility boundaries avoid duplication.

## Subtasks

## Notes / Log
