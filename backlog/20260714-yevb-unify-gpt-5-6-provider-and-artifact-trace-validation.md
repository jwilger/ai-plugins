---
title: Unify GPT-5.6 provider and artifact trace validation
blocked_by: []
blocks: []
tags: [minor, evals, architecture, trace, maintainability]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the GPT-5.6 provider wrapper and post-run isolation checker call one shared complete trace-validation function.

## Context / Why

Verified MINOR architecture finding from 20260709-spx8: both enforcement boundaries independently repeat item, notification, raw-response-item, and server-request validation while trace-policy.mjs shares only allowlists/helpers, leaving lifecycle rules vulnerable to drift.

## Acceptance criteria

## Subtasks

## Notes / Log
