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
