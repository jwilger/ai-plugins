---
title: Synchronize documented Promptfoo pins with the package source of truth
blocked_by: []
blocks: []
tags: [evals, docs, release-integration, promptfoo, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Keep operator-facing Promptfoo version guidance synchronized with the package source of truth.

## Context / Why

Deferred MINOR from final review of 20260709-spx8. package.json now pins Promptfoo 0.121.18 for GPT-5.6 recognition and billing metadata, while the root README, agentic-systems-engineering README, and consuming-project setup text still name 0.121.17. The duplicate literals can send maintainers back to the unsupported behavior.

## Acceptance criteria

- [x] Root README dependency and consuming-project setup guidance match the Promptfoo version pinned in package.json.
- [x] Agentic-systems-engineering guidance and any scaffold or launcher diagnostics either match the package pin or avoid duplicating a literal version.
- [x] A focused consistency check fails when operational Promptfoo version guidance drifts from the package source of truth.

## Subtasks

## Notes / Log

- 2026-07-13: Final-review iteration 1 found the remaining scaffold skill, scaffold reference, and promptfoo-mcp diagnostic pin drift blocks the current GPT-5.6 migration. 20260709-spx8 will address these caused release-integration paths now; retain this item until that work is verified and landed.
- 2026-07-14: Backlog grooming 2026-07-14: Closed as satisfied by 20260709-spx8 and subsequent landed corrections. package.json, the root README, agentic-systems-engineering scaffold guidance, the promptfoo-mcp diagnostic, and focused consistency tests now use Promptfoo 0.121.18. Remaining 0.121.17 references are explicitly historical benchmark evidence rather than current operator guidance.
