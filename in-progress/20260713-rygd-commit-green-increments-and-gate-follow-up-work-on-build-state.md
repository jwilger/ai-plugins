---
title: Make final review risk-proportionate and CI-driven
blocked_by: []
blocks: []
tags: [development-discipline, final-review, workflow, risk, ci, policy, bug, major, top-priority]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
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
- [ ] Deferred and already-known findings are not re-verified or re-reported without materially severity-changing evidence, and they do not block completion or reset clean state when the reviewed diff hash is unchanged and required ticket/report evidence exists.
- [ ] After a blocking fix, high-risk review reruns only affected lenses plus one integration or correctness lens; unaffected lenses are not restarted.
- [ ] Verifier assignments are created only for blocking, disputed, or materially uncertain findings and never merely because a routine MINOR finding was deferred.
- [ ] One shared test-evidence run is recorded per reviewed diff for all lenses to consume; a lens reruns a broad suite only with a documented lens-specific reason.
- [ ] Medium-risk review has a roughly 60–90 minute budget checkpoint that forces an explicit ship, split, or escalate decision while never silently omitting a known blocker.
- [ ] If a ticket grows into a new subsystem or an unusually broad diff, readiness stops final review and requires independently shippable ticket splits.
- [ ] The final-review MCP/coordinator, state schema, disposition routing, lens assignment, targeted rerun logic, skill guidance, and fixtures implement these policies rather than relying on caller prose.
- [ ] Automated tests cover low, medium, high, and exceptional risk; shared evidence reuse; caused versus incidental findings at every severity; unchanged-diff deferral; verifier eligibility; targeted post-fix reruns; review-budget decisions; and oversized-ticket splitting.
- [ ] The coordinator deterministically compiles the risk scout's evidenced per-lens matrix into contract-bound selected lenses and per-lens pass counts: low has zero or one targeted lens once, medium selected lenses once, high applicable broad lenses once, and only explicitly evidenced exceptional dimensions receive a second independent pass by default.
- [ ] After each review-response content change clears fast tests, lightweight review, push, and the CI running-or-green gate, one delta risk assessment compares the old and new diff; it may add coverage or invalidate evidence but cannot erase unresolved blockers or silently reduce required coverage, while unchanged-diff disposition work triggers no replanning.
- [ ] Failed CI and unmet ticket acceptance criteria remain independent completion gates; among review findings, only caused or worsened CRITICAL/MAJOR security findings block, while every non-security finding and incidental/pre-existing security finding is consolidated into backlog work (TRIVIAL remains report-only) unless an explicit release-security stop applies.
- [ ] Discovery saturation uses two consecutive independent samples over the same diff: the risk scout is sample one and selected deep reviewers are the confirmation sample; completion requires the confirmation sample to add no semantically new MAJOR/CRITICAL failure path, and any newly discovered path triggers only affected-lens confirmation until a sample adds none.
- [ ] Initial final-review planning returns exactly one fresh, diff-bound broad-but-shallow risk-scout assignment before any deep-review lens assignments; it covers every review dimension, consumes shared evidence, records canonical semantic failure paths for discovery sample one, and cannot run tests, invoke verifiers, or recurse into more planning.
- [ ] After this policy ships with green CI, the entire existing backlog is deduplicated, consolidated, and reprioritized at the ticket boundary using value, risk/impact, likelihood or observed frequency, and opportunity cost; repeated sightings update likelihood evidence on the existing ticket rather than creating duplicates.

## Subtasks

## Notes / Log
