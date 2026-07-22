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

- [ ] Projects can configure a maximum queued-ticket count, with documented migration and default behavior when the setting is absent.
- [ ] The configuration clearly defines which ticket statuses count toward the limit.
- [ ] Creating, reopening, or moving a ticket into any counted status refuses admission when it would exceed the configured limit.
- [ ] A refusal is actionable and tells the user to replace a lower-value ticket, combine overlapping work, or reject the candidate.
- [ ] CLI, MCP, dashboard, and every other ticket mutation surface share the same enforcement behavior.
- [ ] Admission enforcement is concurrency-safe so simultaneous successful mutations cannot exceed the configured limit.
- [ ] Automated tests cover configuration, counted statuses, every admission path, refusals, defaults or migration, and simultaneous admissions.
- [ ] User and operator documentation explains configuration, counted statuses, refusals, migration/default behavior, and recovery.

## Subtasks

## Notes / Log
