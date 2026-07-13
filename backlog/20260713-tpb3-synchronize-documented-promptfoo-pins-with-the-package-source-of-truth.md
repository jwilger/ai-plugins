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

- [ ] Root README dependency and consuming-project setup guidance match the Promptfoo version pinned in package.json.
- [ ] Agentic-systems-engineering guidance and any scaffold or launcher diagnostics either match the package pin or avoid duplicating a literal version.
- [ ] A focused consistency check fails when operational Promptfoo version guidance drifts from the package source of truth.

## Subtasks

## Notes / Log
