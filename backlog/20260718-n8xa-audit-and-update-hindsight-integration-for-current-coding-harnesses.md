---
title: Audit and update Hindsight integration for current coding harnesses
blocked_by: []
blocks: []
tags: [hindsight, memory, codex, claude-code, plugins, maintenance]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Audit every marketplace plugin and harness-facing integration that configures or documents Hindsight, then update it to the supported Hindsight version's official integration guidance for each current coding harness.

## Context / Why

For each supported coding harness (including Codex and Claude Code where applicable), identify the installed/supported Hindsight version and use the official Hindsight documentation for that exact version as the source of truth. Verify memory-bank selection/isolation, lifecycle hooks, automatic recall/retain behavior, MCP configuration, budgets/filters, credentials and secret handling, plugin documentation, and upgrade behavior. Preserve harness-specific differences rather than forcing one shared configuration. Add focused tests or smoke checks for configuration syntax, hook execution, bank routing, recall, retention, and failure behavior. Record version/source provenance and any intentionally unsupported features. Run the repository's required plugin behavior evals and CI gates before completion.

## Acceptance criteria

## Subtasks

## Notes / Log
