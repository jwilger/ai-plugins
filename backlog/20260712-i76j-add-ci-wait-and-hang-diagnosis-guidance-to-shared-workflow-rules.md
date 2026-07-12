---
title: Add CI wait and hang-diagnosis guidance to shared workflow rules
blocked_by: []
blocks: []
tags: [ci, workflow, guidance]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Document project-agnostic guidance for waiting on CI: use recent successful run duration as a baseline; do not call a normal full gate blocked merely because it is still running; if a run exceeds the comparable baseline by roughly five minutes without a change that plausibly explains the increase, inspect the active step/logs and consider cancelling the run before retrying.

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log
