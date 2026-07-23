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

- [x] The dependency audit no longer reports the supported fast-uri, sharp/libvips, fast-xml-parser, or @hono/node-server advisories.
- [x] Repository checks and provider-backed evaluation behavior pass with the resolved dependency graph.

## Subtasks

## Notes / Log

- 2026-07-23: Delivered 8b4ee20d6649dad032391de3e3bcd24756476f7e to main. Owner-scoped overrides resolve @hono/node-server 2.0.11, fast-uri 4.1.1, and sharp 0.35.3; fast-xml-parser resolves in-range at 5.10.1. npm audit reports zero vulnerabilities. Full local gate passed (274 development-discipline Rust tests, 44 Tiber mutants, 574 Bats), focused Claude+Codex eval eval-Pqe passed 2/2, final review completed clean, and GitHub CI run 30033583731 reached terminal success.
