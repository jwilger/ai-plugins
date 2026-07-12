---
title: Add development workflow router skill
blocked_by: [20260708-dsfg-add-cross-project-change-preflight-skill-to-development-discipline]
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a compact development-workflow router skill to the development-discipline plugin so installed marketplace sessions classify development tasks, inspect repo state, and invoke the existing TDD, debugging, verification, final-review, review-feedback, PR babysitting, docs, security, OpenAI, and browser-verification skills at the right lifecycle points without duplicating their full instructions.

## Context / Why

## Acceptance criteria

- [ ] development-discipline includes a new development-workflow router skill that triggers for development tasks from initial request through implementation, PR, CI, review, and merge readiness.
- [ ] The router skill delegates detailed mechanics to existing skills including test-driven-development, systematic-debugging, verification-before-completion, final-review, receiving-code-review, babysit-pr, and relevant external documentation/security/GitHub/OpenAI/browser skills.
- [ ] development-discipline README, plugin manifests, marketplace metadata, and root catalog are updated consistently, including the appropriate semver bump.
- [ ] Behavior fixtures cover normal implementation routing, CI failure routing, and PR-to-merge routing through the full-marketplace eval surface.
- [ ] The change does not duplicate the full 17-step workflow across every plugin and does not embed project-specific implementation rules that belong in a consuming repo's AGENTS.md.

## Subtasks

## Notes / Log
