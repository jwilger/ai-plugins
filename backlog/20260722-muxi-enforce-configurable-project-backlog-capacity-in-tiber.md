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

## Subtasks

## Notes / Log
