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

This matters because tautological checks consume maintenance time and evaluation tokens while creating false confidence. A useful test fails when the application or library behaves incorrectly, not when documentation wording or workflow layout changes. Implementation notes: never add a test whose only behavior is reading a repository-owned file and asserting that it contains a specific string. The sole exception is testing a program whose actual responsibility is to create or edit that file; assert against that program's generated output. Do not add tests for continuous integration workflow definitions or job structure; execution of the workflow in continuous integration is the test. When an existing test matches either anti-pattern in any project, remove it or replace it with a public, black-box test of application or library behavior. Agent-guidance changes may use semantic provider-backed evaluations. If there is no meaningful product behavior to exercise, do not invent a test.

## Acceptance criteria

- [ ] Agent-guidance changes use semantic behavior evaluations when behavior needs regression coverage, without adding phrase-presence tests as a fast substitute.
- [ ] New tests are justified by a user- or caller-observable behavior of the application or library; when no meaningful behavior exists to exercise, no tautological test is added.
- [ ] Standing development guidance for every project forbids tests that only read committed repository files and check for specific strings, except when testing a program that creates or edits that file.
- [ ] Standing development guidance forbids tests of continuous integration workflow definitions or job structure; executing the workflow in continuous integration is the test.
- [ ] When an existing test matching either anti-pattern is found in any project, it is removed or replaced with a test of observable application or library behavior.

## Subtasks

## Notes / Log
