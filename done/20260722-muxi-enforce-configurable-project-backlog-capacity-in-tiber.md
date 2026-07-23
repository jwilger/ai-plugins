---
title: Enforce configurable project backlog capacity in Tiber
blocked_by: []
blocks: []
tags: [tiber, backlog, capacity, concurrency, configuration]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a project-configurable maximum for queued tickets and enforce it atomically across every Tiber mutation surface so concurrent admissions cannot exceed capacity.

## Context / Why

Prompt guidance cannot prevent every caller or simultaneous write from overfilling a project backlog. Define the counted statuses and enforce the project limit when creating, reopening, or moving tickets into them across CLI, MCP, dashboard, and other mutation paths. Refusals must tell users to replace, combine, or reject work. Preserve compatible defaults for projects without the setting, document migration, and decide whether the replenishment threshold belongs in Tiber configuration or remains repository SOP.

## Acceptance criteria

- [x] Projects can configure a maximum queued-ticket count, with documented migration and default behavior when the setting is absent.
- [x] The configuration clearly defines which ticket statuses count toward the limit.
- [x] Creating, reopening, or moving a ticket into any counted status refuses admission when it would exceed the configured limit.
- [x] A refusal is actionable and tells the user to replace a lower-value ticket, combine overlapping work, or reject the candidate.
- [x] CLI, MCP, dashboard, and every other ticket mutation surface share the same enforcement behavior.
- [x] Admission enforcement is concurrency-safe so simultaneous successful mutations cannot exceed the configured limit.
- [x] Automated tests cover configuration, counted statuses, every admission path, refusals, defaults or migration, and simultaneous admissions.
- [x] User and operator documentation explains configuration, counted statuses, refusals, migration/default behavior, and recovery.
- [x] The design explicitly decides whether the replenishment threshold belongs in Tiber configuration or remains SOP-only, with rationale.
- [x] Before admitting a candidate, guidance requires checking completed, abandoned, and current tickets for the same root outcome; reworded duplicates are combined or rejected instead of consuming backlog capacity.

## Subtasks

## Notes / Log

- 2026-07-23: Delivered Tiber 0.11.0 backlog capacity enforcement through main at 9070292f. Added repository-owned max_queued configuration, backlog-only counting, CLI/MCP admission refusal for create and backlog transitions, cross-clone CAS revalidation with bounded retries, and rollback that preserves unrelated local-only tasks and prior transition status. Documented replace/combine/reject handling, migration/recovery, dashboard count-neutral priority behavior, and SOP-only replenishment threshold. Verification: full serialized just ci passed (44 mutation classifications and 574 Bats tests), final targeted Claude+Codex behavior eval eval-3Uo-2026-07-23T16:24:32 passed 2/2, release artifacts/checksums rebuilt, final review clean with bounded retry-load risk accepted as proportionate, and GitHub CI run 30025775123 reached terminal success for 9070292f.
