---
title: Keep README plugin catalog versions synchronized with manifests
blocked_by: []
blocks: []
tags: [marketplace, documentation, versions, validation, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the README harness catalog versions derive from or validate against plugin manifests so plugin releases cannot leave user-facing version rows stale.

## Context / Why

MINOR caused finding from the 20260709-spx8 review. README currently advertises pre-bump versions for agentic-systems-engineering, advisor, babysit-pr, and tiber even though their synchronized manifests/marketplace entries have advanced. Add an automated sync/validation path and repair the rows in that focused ticket.

## Acceptance criteria

## Subtasks

## Notes / Log
