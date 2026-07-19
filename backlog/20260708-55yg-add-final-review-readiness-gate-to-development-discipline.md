---
title: Check required evidence before final review begins
blocked_by: []
blocks: []
tags: [development-discipline, final-review, readiness, artifacts, release-integration, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a readiness check that confirms the ticket’s acceptance evidence and related deliverables are present before formal review starts. Missing tests, documentation, versions, generated files, evaluations, release artifacts, or tracked blockers should be identified early with specific next steps.

## Context / Why

Implementation notes:\n\nThe risk-proportionate policy already requires the ticket's actual acceptance criteria to be implemented before final review. This ticket does not redefine, waive, or invent those criteria. Its remaining responsibility is a mechanically represented evidence inventory bound to the exact work item and diff, covering cross-surface obligations that are otherwise discovered late. Accepted defenses and externally tracked blockers continue through the existing prior_defenses and authoritative-state contract.

## Acceptance criteria

- [ ] Before final_review.assess or final_review.plan, the enforced workflow produces a readiness result bound to the exact work item, baseline, changed-file inventory, and diff hash.
- [ ] The result verifies evidence for the ticket's existing acceptance criteria and inventories changed or generated artifacts, schemas, versions and manifests, documentation, required tests and evals, release artifacts, and external blockers without inventing new requirements.
- [ ] Missing required evidence or an actionable omitted surface stops planning with specific remediation; a manual prose fallback cannot satisfy the enforced readiness gate.
- [ ] Accepted defenses and tracked blockers reuse the existing prior_defenses and authoritative-state contract; no parallel carry-forward mechanism is introduced.
- [ ] Focused coordinator tests and behavior fixtures cover ready, unmet-acceptance, missing-generated-artifact, version or documentation drift, required-eval omission, and defended or externally blocked cases.

## Subtasks

## Notes / Log

- 2026-07-14: Backlog grooming 2026-07-14: Narrowed after 20260713-rygd. The active policy already requires actual acceptance criteria before final review; this ticket now owns only the missing mechanically bound evidence and cross-surface readiness inventory.
