---
title: Keep final-review risk assessments connected to their review session
blocked_by: []
blocks: [20260707-c2bu-strengthen-engineering-standards-against-primitive-obsession]
tags: [development-discipline, final-review, bug]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Prevent valid final reviews from failing because the risk assessment identity no longer matches the review session that requested it.

## Context / Why

A final_review.plan call in a separate repository rejected an apparently unchanged risk-scout result with `risk_assessment_assignment_id_mismatch=true`. The supplied assessment included its assignment ID, final-review-scoped subagent key, shared evidence identity, and caller attestation. Investigate how assignment identity is issued, retained, and compared across assess/attest/plan calls; make same-session results reliable while preserving rejection of stale or cross-session assessments. Diagnostics must remain actionable without exposing private review state.

## Acceptance criteria

- [x] A risk assessment returned and attested through the supported final-review workflow is accepted by `final_review.plan` when reused unchanged in the same review session.
- [x] Stale, altered, or cross-session risk assessments remain rejected, with a diagnostic that distinguishes the identity problem and identifies the required recovery action without exposing sensitive internal state.
- [x] Automated behavior or integration coverage reproduces the assess/attest/plan call shape, including assignment ID, subagent key, and caller attestation, and prevents this mismatch regression.

## Subtasks

## Notes / Log

- 2026-07-21: Reproduced during HCQ6 final review. A restarted assess/plan session returned `risk_assessment_assignment_id_mismatch=true` when the plan call's shared-evidence command labels differed from assess; retrying with byte-for-byte contract fields cleared that error. The same valid scout result was then rejected as `risk_assessment_low_profile_too_many_lenses max=1`: the coordinator-required scout assessed all nine assigned dimensions as low, but plan's low-profile validator permitted at most one dimension. Coverage should include both identity binding diagnostics and consistency between scout output requirements and low-risk plan validation.
- 2026-07-22: Delivered commit 0b1e73a to main. Formal final-review coordinator 0.15.4 accepted the unchanged same-session all-dimension low-risk assessment, selected exactly one deterministic correctness lens, received a clean reviewer result, and reached terminal complete. Local `nix develop -c just ci` passed (552 Bats tests, 269 Rust tests, mutation 38 caught/6 unviable, release reconstruction/checksums and remaining gates green). Marketplace canary initially missed eval-case-reporter in one Codex sample; an unchanged isolated rerun passed both Codex and Claude 2/2. Targeted final-review provider evals exposed rubric overreach and are captured separately in 20260722-7hf5. GitHub Actions run 29884415362 is in progress for the exact pushed SHA.
