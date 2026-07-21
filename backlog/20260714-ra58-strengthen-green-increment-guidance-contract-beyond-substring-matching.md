---
title: Test real behavior instead of repository text
blocked_by: []
blocks: []
tags: [development-discipline, tests, test-quality, high-priority, guardrails]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Tests should prove behavior that users or callers can observe. Tests that only open a committed file and check whether it contains a particular phrase are brittle and do not prove that the product works. Tests that inspect continuous integration workflow files have the same problem because running those workflows is already the meaningful test. Establish this as a standing rule for every project, and remove or replace existing tests that violate it when they are found.

## Context / Why

This matters because tautological checks consume maintenance time and evaluation tokens while creating false confidence. A useful test fails when the application or library behaves incorrectly, not when documentation wording or workflow layout changes. Implementation notes: prefer testing the end-user-visible behavioral effect produced by a file. Never add a test whose only behavior is reading a repository-owned file and asserting that it contains a specific string. File-text inspection is allowed only when the program creates or edits that file and no behavioral-effect test can prove the requirement; even then, assert against the program's generated output, never a pre-existing committed fixture or policy file. Do not add tests for continuous integration workflow definitions or job structure; execution of the workflow in continuous integration is the test. When an existing test matches either anti-pattern in any project, remove it or replace it with a public, black-box test of application or library behavior. Agent-guidance changes may use semantic provider-backed evaluations. If there is no meaningful product behavior to exercise, do not invent a test.

## Acceptance criteria

- [ ] Agent-guidance changes use semantic behavior evaluations when regression coverage is useful, without adding phrase-presence tests as a fast substitute.
- [ ] Every new automated test is justified by user- or caller-observable application or library behavior; no test is invented when there is no meaningful behavior to exercise.

## Subtasks

## Notes / Log
