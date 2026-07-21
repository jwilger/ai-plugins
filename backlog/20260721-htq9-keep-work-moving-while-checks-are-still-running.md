---
title: Keep work moving while checks are still running
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Agents sometimes pause implementation merely because the latest continuous integration run has not finished, even though no failure exists. Agents should continue the next safe, test-driven increment whenever the latest pushed build is running or successful, and stop unrelated work only after an actual failed build. This avoids unnecessary delivery delays while preserving focused failure recovery.

## Context / Why

The test-driven-development skill already says to continue when continuous integration is running or green unless a prior failure hold exists. Strengthen the executable guidance, behavior fixtures, or workflow enforcement so agents reliably follow that rule. A queued or in-progress run is not a failure hold. Only a completed failed build invokes ci-failure-follow-up and blocks follow-up implementation, review remediation, or a new ticket.

## Acceptance criteria

- [ ] When the latest pushed build is queued or in progress and no earlier failure hold exists, the agent continues the next safe test-driven increment instead of waiting.

## Subtasks

## Notes / Log
