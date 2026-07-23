---
title: Resolve the remaining dependency security warnings
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

The repository still reports four dependency security warnings after the recent js-yaml remediation. Resolve the remaining supported warnings together so maintainers have one coherent, verified dependency update instead of several symptom-level tickets.

## Context / Why

Users and maintainers should be able to rely on a clean dependency audit without accepting avoidable known vulnerabilities. Investigate the affected fast-uri, sharp/libvips, fast-xml-parser, and @hono/node-server dependency paths, update or replace them safely, preserve Promptfoo and evaluation behavior, and document any warning that cannot be resolved safely.

## Acceptance criteria

- [ ] The dependency audit no longer reports the supported fast-uri, sharp/libvips, fast-xml-parser, or @hono/node-server advisories.

## Subtasks

## Notes / Log
