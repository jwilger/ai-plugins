---
title: Add an artifact-obligation readiness gate before final-review planning
blocked_by: []
blocks: []
tags: [development-discipline, final-review, readiness, artifacts, release-integration, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a scope-bound readiness result that verifies required acceptance evidence and catches missing schema, version, documentation, generated-artifact, eval, release, or blocker work before the risk scout and formal review begin.

## Context / Why

The risk-proportionate policy already requires the ticket's actual acceptance criteria to be implemented before final review. This ticket does not redefine, waive, or invent those criteria. Its remaining responsibility is a mechanically represented evidence inventory bound to the exact work item and diff, covering cross-surface obligations that are otherwise discovered late. Accepted defenses and externally tracked blockers continue through the existing prior_defenses and authoritative-state contract.

## Acceptance criteria

- [ ] Final-review guidance requires a local readiness checklist before the multi-lens review loop begins.
- [ ] The change includes eval cases that exercise final-review readiness and defended-finding carry-forward behavior.
- [ ] Before final_review.plan, the workflow produces a readiness result bound to the exact current scope and task and stops with actionable remediation when required work is missing.
- [ ] Readiness checks ticket acceptance criteria, changed and generated artifacts, schemas, versions/manifests, documentation, required tests/evals, release artifacts, and external blockers.

## Subtasks

## Notes / Log
