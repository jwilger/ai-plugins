---
title: Bring Hindsight memory setup up to date for each coding tool
blocked_by: []
blocks: []
tags: [hindsight, memory, codex, claude-code, plugins, maintenance]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Review every Hindsight memory integration in the marketplace and update it to the official guidance for the supported version of each coding tool. Verify that memory separation, session hooks, recall, retention, credentials, and failure behavior work as intended.

## Context / Why

For each supported coding harness (including Codex and Claude Code where applicable), identify the installed/supported Hindsight version and use the official Hindsight documentation for that exact version as the source of truth. Verify memory-bank selection/isolation, lifecycle hooks, automatic recall/retain behavior, MCP configuration, budgets/filters, credentials and secret handling, plugin documentation, and upgrade behavior. Preserve harness-specific differences rather than forcing one shared configuration. Add focused tests or smoke checks for configuration syntax, hook execution, bank routing, recall, retention, and failure behavior. Record version/source provenance and any intentionally unsupported features. Run the repository's required plugin behavior evals and CI gates before completion.

## Acceptance criteria

- [ ] An inventory identifies every Hindsight integration/configuration/documentation surface by coding harness and the exact supported Hindsight version.
- [ ] Each harness integration matches the official Hindsight documentation for its supported version, with versioned source provenance recorded in repository documentation.
- [ ] Bank selection and isolation, lifecycle hooks, recall/retain behavior, MCP settings, budgets/filters, and secret handling are verified for each supported harness.
- [ ] Focused automated tests or smoke checks cover configuration validity, hook execution, bank routing, targeted recall, session retention, and safe failure behavior.
- [ ] Harness-specific differences and intentionally unsupported Hindsight features are documented; shared configuration does not erase required differences.
- [ ] All affected plugin versions and marketplace metadata are bumped consistently, and required behavior evals plus `just ci` pass.

## Subtasks

## Notes / Log
