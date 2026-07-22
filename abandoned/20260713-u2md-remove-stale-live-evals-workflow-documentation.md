---
title: Remove stale live-evals workflow documentation
blocked_by: []
blocks: []
tags: [documentation, github-actions, evals, authentication, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Remove or replace README and AGENTS guidance for the deleted live-evals.yml workflow and clarify which credentials each remaining eval path actually uses.

## Context / Why

Pre-existing MINOR found during 20260709-spx8 docs review. README claims a provider-backed live workflow requires OPENAI_API_KEY and ANTHROPIC_API_KEY, and AGENTS.md names .github/workflows/live-evals.yml, but that workflow was deleted in commit 30ad122 and Codex local runs use local auth. Reconcile current CI/workflow inventory and auth documentation.

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Release, tooling, or maintenance convenience with lower current blocking impact and value-to-cost than the retained defects and security update. Rejected without an overflow or shadow backlog.
