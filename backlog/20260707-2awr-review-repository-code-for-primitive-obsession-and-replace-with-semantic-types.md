---
title: Review repository code for primitive obsession and replace with semantic types
blocked_by: [20260707-c2bu-strengthen-engineering-standards-against-primitive-obsession]
blocks: []
tags: [refactor, semantic-types, engineering-standards, technical-debt]
---

## Summary

Inventory every repository domain boundary and replace primitive-obsessed domain data with named semantic types in bounded component slices while preserving observable behavior.

## Context / Why

This is a repository-wide implementation task, not only a report, but it must proceed as a reviewable sequence. First inventory code-bearing plugins/crates/scripts and distinguish domain logic from I/O adapters, serialization DTOs, configuration, shell glue, and static presentation. Then remediate each component independently under the strengthened standard from 20260707-c2bu. Do not mechanically wrap every incidental primitive: every remaining primitive must either be confined to a documented boundary or represented by an invariant-carrying domain type.

## Acceptance criteria

- [ ] A checked repository inventory identifies each domain boundary and primitive-obession violation by component, with reasoned exclusions for I/O adapters, DTOs, configuration, shell glue, and presentation-only data.
- [ ] Every inventoried domain violation is assigned to and completed in a bounded component slice; no raw domain primitive remains untracked.
- [ ] Each slice introduces named types with private or invariant-safe construction, parses at the boundary, keeps primitives in adapters, and preserves public behavior with black-box tests.

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: perform a thorough repository-wide code review looking for primitive obsession, then fix it by introducing and using named semantic types for all domain data per engineering-standards. This is an implementation task, not just a report; primitives should be confined to I/O boundaries and parsed into semantic types before entering domain logic.
