---
title: Require meaningful domain types instead of renamed basic values
blocked_by: []
blocks: [20260707-2awr-review-repository-code-for-primitive-obsession-and-replace-with-semantic-types]
tags: [engineering-standards, semantic-types, guidance, evals]
---

## Summary

Clarify the engineering standard so a renamed string, number, or similar basic value does not count as a meaningful domain type. Guidance and examples should show how named types can enforce valid business values as data enters the system.

## Context / Why

Implementation notes: The current rule already says primitives, built-ins, and structural types belong only at I/O boundaries, but prior implementation behavior still treated raw String values and aliases such as type UserId = String as domain modeling. Narrow this task to the missing clarification: semantic types are named invariant-carrying wrappers or sum types constructed by parsing at the boundary, not aliases that merely rename primitive representation. Update the reusable engineering-standards skill and canonical rule together before the repository-wide remediation task 20260707-2awr.

## Acceptance criteria

- [ ] Canonical rules explicitly state that raw strings, numbers, booleans, UUIDs, built-ins, structural records, and type aliases over them are not semantic domain types.
- [ ] Guidance demonstrates named wrappers and sum types with invariant-safe construction, immediate boundary parsing, and no downstream revalidation.
- [ ] docs/rules/semantic-types.md, the engineering-standards skill, related documentation/metadata, and the required plugin version remain consistent.
- [ ] Behavior fixtures reject raw String/domain APIs and structural aliases such as type UserId = string, then recommend a named parsed semantic type.

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: strengthen the engineering-standards plugin/docs around zero primitive obsession so code like the PR 40 example is not written. Make the standard explicit that primitives and structural aliases are not semantic domain types; String is not a semantic type. Guidance should push agents toward named semantic types at boundaries and throughout the domain, consistent with parse-dont-validate.
