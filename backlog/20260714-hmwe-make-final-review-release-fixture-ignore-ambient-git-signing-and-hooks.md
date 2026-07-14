---
title: Make final-review release fixture ignore ambient Git signing and hooks
blocked_by: []
blocks: []
tags: [development-discipline, tests, git, hermeticity, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the temporary Git fixture used by the development-discipline source/release parity flow independent of ambient signing and hook configuration.

## Context / Why

Lightweight review of the 639d220 CI repair found that the fixture's empty commit inherits global commit.gpgSign and core.hooksPath settings. On a developer machine this can fail or hang an otherwise hermetic release check. Disable signing and hooks for that one temporary-repository commit, and add focused coverage if practical. This is a MINOR local-tooling reliability issue with no security or human-safety impact.

## Acceptance criteria

## Subtasks

## Notes / Log
