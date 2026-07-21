---
title: Update brace-expansion to prevent extremely slow pattern processing
blocked_by: []
blocks: []
tags: [security, dependencies, high-priority]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Update the locked brace-expansion dependency so specially shaped patterns cannot consume exponential processing time.

## Context / Why

GitHub Dependabot alert 4 reports a high-severity denial-of-service issue in brace-expansion through package-lock.json. Upgrade to brace-expansion 2.1.2 or newer through the normal dependency graph, confirm the lockfile remains valid, run the repository's required checks, and verify the alert is resolved. Keep this separate from the currently active development-workflow ticket. Alert: https://github.com/jwilger/ai-plugins/security/dependabot/4

## Acceptance criteria

- [ ] The affected nested brace-expansion dependency resolves to version 2.1.2 or newer.
- [ ] npm audit no longer reports GHSA-3jxr-9vmj-r5cp.

## Subtasks

## Notes / Log
