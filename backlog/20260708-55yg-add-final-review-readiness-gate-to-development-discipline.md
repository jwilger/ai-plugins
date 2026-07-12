---
title: Add final-review readiness gate to development-discipline
blocked_by: []
blocks: []
tags: [development-discipline, final-review, readiness, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a pre-final-review readiness result that catches unmet ticket criteria and missing schema, version, documentation, generated-artifact, eval, release, or blocker work before the expensive review loop begins.

## Context / Why

Current final-review behavior already carries defended findings through the MCP prior_defenses contract, so this task must not invent a parallel exception system. The remaining gap is an explicit readiness check before final_review.plan that is bound to the current scope and task, stops on actionable omissions, and passes known defended findings or externally tracked blockers through the existing contract.

## Acceptance criteria

- [ ] Final-review guidance requires a local readiness checklist before the multi-lens review loop begins.
- [ ] The change includes eval cases that exercise final-review readiness and defended-finding carry-forward behavior.
- [ ] Before final_review.plan, the workflow produces a readiness result bound to the exact current scope and task and stops with actionable remediation when required work is missing.
- [ ] Readiness checks ticket acceptance criteria, changed and generated artifacts, schemas, versions/manifests, documentation, required tests/evals, release artifacts, and external blockers.
- [ ] Accepted defenses and tracked blockers reuse the existing prior_defenses/state contract; no duplicate carry-forward mechanism is introduced.

## Subtasks

## Notes / Log
