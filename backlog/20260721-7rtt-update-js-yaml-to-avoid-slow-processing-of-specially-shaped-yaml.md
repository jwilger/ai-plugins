---
title: Update js-yaml to avoid slow processing of specially shaped YAML
blocked_by: []
blocks: []
tags: [security, dependencies]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Update the locked js-yaml dependency to a fixed version so specially shaped YAML cannot consume excessive processing time.

## Context / Why

GitHub Dependabot alert 3 reports a medium-severity denial-of-service issue in js-yaml versions 5.0.0 through 5.2.0, referenced by package-lock.json. Upgrade to js-yaml 5.2.1 or newer, confirm the dependency graph and lockfile remain valid, run the repository's required checks, and close or resolve the alert through the normal dependency update. Alert: https://github.com/jwilger/ai-plugins/security/dependabot/3

## Acceptance criteria

## Subtasks

## Notes / Log
