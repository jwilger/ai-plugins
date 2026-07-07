---
title: Encode MCP bootstrap fix in marketplace plugins
blocked_by: [20260707-55ky-fix-codex-mcp-startup-failures-for-tiber-and-promptfoo]
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

After the tiber and promptfoo MCP startup failures are actually fixed, encode the necessary bootstrap into the marketplace plugins themselves. Fresh plugin installs should set up the expected executable paths, dependency restore, or launcher indirection needed by Codex instead of relying on one-off local repair.

## Context / Why

## Acceptance criteria

- [ ] The relevant plugin manifests, launchers, scripts, or README bootstrap instructions include the durable fix required for tiber and promptfoo MCP startup.
- [ ] A clean install or bootstrap path reproduces the fixed state without manual local-only commands.

## Subtasks

## Notes / Log

- 2026-07-07: Implemented locally as stacked branch mcp-bootstrap-upgrade-docs at commit 66c152e, based on mcp-startup-direct-launchers. Documents upgrading to agentic-systems-engineering 0.1.4 and tiber 0.2.3 as the marketplace bootstrap path. Verification run: nix develop -c just validate-marketplace.
