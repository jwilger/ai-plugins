---
title: Require meaningful domain types instead of renamed basic values
blocked_by: [20260721-mvj4-keep-final-review-risk-assessments-connected-to-their-review-session]
blocks: [20260707-2awr-review-repository-code-for-primitive-obsession-and-replace-with-semantic-types]
tags: [engineering-standards, semantic-types, guidance, evals]
claim:
  host: unknown
  session: unknown
---

## Summary

Clarify the engineering standard so a renamed string, number, or similar basic value does not count as a meaningful domain type. Guidance and examples should show how named types can enforce valid business values as data enters the system.

## Context / Why

Implementation notes: The current rule already says primitives, built-ins, and structural types belong only at I/O boundaries, but prior implementation behavior still treated raw String values and aliases such as type UserId = String as domain modeling. Narrow this task to the missing clarification: semantic types are named invariant-carrying wrappers or sum types constructed by parsing at the boundary, not aliases that merely rename primitive representation. Update the reusable engineering-standards skill and canonical rule together before the repository-wide remediation task 20260707-2awr.

## Acceptance criteria

- [x] Canonical rules explicitly state that raw strings, numbers, booleans, UUIDs, built-ins, structural records, and type aliases over them are not semantic domain types.
- [x] Guidance demonstrates named wrappers and sum types with invariant-safe construction, immediate boundary parsing, and no downstream revalidation.
- [x] docs/rules/semantic-types.md, the engineering-standards skill, related documentation/metadata, and the required plugin version remain consistent.
- [x] Behavior fixtures reject raw String/domain APIs and structural aliases such as type UserId = string, then recommend a named parsed semantic type.

## Subtasks

## Notes / Log

- 2026-07-07: Requirement detail: strengthen the engineering-standards plugin/docs around zero primitive obsession so code like the PR 40 example is not written. Make the standard explicit that primitives and structural aliases are not semantic domain types; String is not a semantic type. Guidance should push agents toward named semantic types at boundaries and throughout the domain, consistent with parse-dont-validate.
- 2026-07-22: Delivered candidate cfc64d3 to main. Exact rebased candidate passed full `nix develop -c just ci` (269 development-discipline tests, mutation 38 caught/6 unviable, release reconstruction, 552 Bats, manifests/formatting and remaining gates). Focused semantic-type provider matrix passed 6/6 across 3 Claude and 3 Codex samples, all score 1 (`evals/out/c2bu-semantic-types-final-matrix/results.json`). Rebuilt final-review coordinator 0.15.4 accepted the complete same-session low-risk assessment, assigned one correctness lens, received a clean result, and reached terminal complete. Full-marketplace canary Claude passed; Codex accurately named/described all plugins but was rejected for capability wording instead of literal skill names, tracked by existing 20260719-tdms with reproduction notes. GitHub Actions run 29886722885 is in progress for exact pushed SHA.
- 2026-07-22: GitHub Actions run 29886722885 reached terminal success for exact SHA cfc64d3: Codex cross-harness manifests, eval config dry-run, Quality gate, and required CI gate all passed.
