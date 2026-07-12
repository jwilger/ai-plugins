---
title: Add development workflow router skill
blocked_by: [20260708-dsfg-add-cross-project-change-preflight-skill-to-development-discipline]
blocks: []
tags: [development-discipline, workflow-router, skills, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a compact development-workflow router that inspects task and repository state, classifies the current lifecycle phase, and routes to existing specialist skills without copying their instructions.

## Context / Why

The router is a lifecycle dispatcher, not a replacement for test-driven-development, systematic-debugging, verification-before-completion, final-review, receiving-code-review, PR/CI monitoring, documentation, security, OpenAI, or browser skills. Define phase precedence and stop/skip rules, respect repository-local instructions, invoke only skills available in the current harness, and explain a safe fallback when a named specialist is unavailable. The cross-project preflight task 20260708-dsfg is a prerequisite so the router does not immediately need a second structural rewrite.

## Acceptance criteria

- [ ] development-discipline includes a new development-workflow router skill that triggers for development tasks from initial request through implementation, PR, CI, review, and merge readiness.
- [ ] The router skill delegates detailed mechanics to existing skills including test-driven-development, systematic-debugging, verification-before-completion, final-review, receiving-code-review, babysit-pr, and relevant external documentation/security/GitHub/OpenAI/browser skills.
- [ ] development-discipline README, plugin manifests, marketplace metadata, and root catalog are updated consistently, including the appropriate semver bump.
- [ ] Behavior fixtures cover normal implementation routing, CI failure routing, and PR-to-merge routing through the full-marketplace eval surface.
- [ ] The change does not duplicate the full 17-step workflow across every plugin and does not embed project-specific implementation rules that belong in a consuming repo's AGENTS.md.

## Subtasks

## Notes / Log
