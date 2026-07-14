---
title: Make final review risk-proportionate and CI-driven
blocked_by: []
blocks: []
tags: [development-discipline, final-review, workflow, risk, ci, policy, bug, major, top-priority]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Implement a risk-proportionate development loop and final-review protocol: preserve green increments through fast tests, lightweight review, commit/push, and CI; select only risk-relevant final lenses; disposition findings by severity and causality; and reserve repeated independent passes for exceptional-risk work.

## Context / Why

The current development-discipline workflow delayed a medium-risk local eval-tooling ticket through repeated broad lens passes, duplicate test execution, routine verification of already-ticketed MINORs, and a hard-coded three-clean-pass protocol. In final-review session 20260709-spx8-gpt56-final-review-20260713-h, all caused findings were MINOR and mapped to existing backlog tickets, yet filter_findings still labeled every one `block`, advance demanded a verifier, and required_clean_iterations=3 prevented completion. This conflicts with the desired inner loop and with proportionate threat/risk modeling. The canonical behavior must live in the actual final-review machinery, skill guidance, fixtures, and tests—not prose alone. This ticket absorbs 20260712-7csp and supersedes the mandatory three-pass/push-blocking policy in 20260711-42si.

## Acceptance criteria

- [ ] Guidance defines full review as the ticket-completion gate, not a prerequisite for preserving a green increment.
- [ ] When full review finds issues, guidance requires a new green tests/light-review commit and push before restarting full review.
- [ ] Before addressing review findings or starting another ticket, guidance requires checking the latest pushed build; running or green permits work, while failed blocks follow-up work until resolved.
- [ ] Full-review instructions pin the baseline commit so pushes during review do not move or erase the reviewed diff.
- [ ] Guidance defines fast unit tests plus lightweight review as the local commit-and-push gate for each implementation increment.
- [ ] Guidance permits longer-running integration, mutation, full-suite, and similarly expensive checks to run in CI instead of blocking each local increment.
- [ ] Final-review planning accepts or derives an explicit risk class from concrete deployment, trust-boundary, reversibility, data, and operational evidence, and enables only lenses justified by that risk.
- [ ] Low-risk work uses lightweight review with at most one optional targeted final lens; medium-risk work uses one targeted full-review pass; high-risk work uses one broad pass.
- [ ] Multiple independent clean passes are available only for exceptional risk such as destructive or irreversible operations, authentication/authorization boundaries, sensitive-data migrations, cryptographic behavior, or safety-critical behavior.
- [ ] A review iteration is clean when no unresolved blocking finding remains after disposition; caused CRITICAL/MAJOR findings block, incidental or pre-existing CRITICAL/MAJOR and all MINOR findings require appropriately prioritized backlog work, and TRIVIAL findings are report-only.

## Subtasks

## Notes / Log
