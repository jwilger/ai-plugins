---
title: Require explicit user approval before amending commits
blocked_by: []
blocks: []
tags: [bug, development-discipline, git, safety]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Treat additive commits as the default repair workflow and require explicit case-by-case user authorization before amending any existing commit.

## Context / Why

A prior PR repair amended a published commit and created avoidable recovery work. Shared/default-branch history must never be rewritten as routine repair. On other branches, an amend is allowed only when the user explicitly authorizes that specific amend; PR feedback and follow-up fixes default to new commits and must not trigger a force-push merely to replace history.

## Acceptance criteria

- [ ] Relevant commit/PR guidance states that amending any commit requires explicit user authorization.
- [ ] Default repair and follow-up workflow uses additive commits and does not force-push solely to replace an amended commit.

## Subtasks

## Notes / Log
