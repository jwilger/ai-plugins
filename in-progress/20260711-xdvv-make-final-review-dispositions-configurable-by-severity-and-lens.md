---
title: Make final-review dispositions configurable by severity and lens
blocked_by: []
blocks: []
tags: [development-discipline, final-review, configuration]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Audit the current final-review severity and disposition behavior, then support reviewer-assigned CRITICAL, MAJOR, MINOR, and TRIVIAL severities that the verifier can challenge or reclassify. Let each project map every severity-and-lens combination to block, ticket, document, or ignore.

## Context / Why

## Acceptance criteria

- [ ] Document which parts of reviewer severity, verifier challenge, and disposition routing already exist before changing behavior.
- [ ] Reviewers assign exactly one of CRITICAL, MAJOR, MINOR, or TRIVIAL to each finding.
- [ ] The verifier can confirm or challenge a finding's assigned severity with an auditable rationale.
- [ ] Project configuration maps each severity-and-lens combination to block, ticket, document, or ignore, with validation and focused tests.

## Subtasks

## Notes / Log

- 2026-07-11: Pre-change audit (main 4eb4c99): reviewer findings currently require severity `error`, `warning`, or `note`; the separate `security_impact` enum is not reviewer severity. Verifier verdicts are `confirmed`, `rejected`, or `uncertain` with rationale only, and do not confirm or change severity. Existing `unrelated_finding_policy` applies only to out-of-scope findings with precedence by_lens → by_severity → default and dispositions `address-now`, `follow-up-ticket`, or `report`. It therefore cannot map the requested per-(review severity, lens) matrix for all findings.
