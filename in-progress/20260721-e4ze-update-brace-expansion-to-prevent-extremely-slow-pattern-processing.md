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

- [x] The affected nested brace-expansion dependency resolves to version 2.1.2 or newer.
- [x] npm audit no longer reports GHSA-3jxr-9vmj-r5cp.
- [ ] A clean dependency install and the full repository CI checks pass.
- [ ] GitHub Dependabot alert 4 is closed or resolved after delivery.

## Subtasks

## Notes / Log

- 2026-07-21: Delivered commit bf7ed8a551f47b105a5673f2f9126fcc71b0df43 to main. The lockfile resolves the affected nested brace-expansion to 2.1.2; npm audit no longer reports GHSA-3jxr-9vmj-r5cp and reports zero high/critical findings. `npm ci` and `nix develop -c just ci` passed, including 586 Bats tests and 44 mutation tests. Final review was clean. GitHub Actions run 29837300426 completed successfully with all four jobs green, and Dependabot alert 4 reports state=fixed.
