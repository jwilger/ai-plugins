---
title: Recommend and configure out-of-sandbox Tiber execution during initialization
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
pr_mr_url: https://github.com/jwilger/ai-plugins/pull/45
pr_mr_status: checks-pending
---

## Summary

Tiber initialization should detect when MCP-backed task writes are likely to fail because Tiber's Git write/sync operations need host access for signed commits, SSH/GPG agents, or push credentials. It should recommend and offer setup for the narrowest safe out-of-sandbox permission: allowing only the Git commands or Tiber-internal Git equivalents required for signed commit-tree/commit and push/sync, rather than running the entire Tiber MCP server or all MCP calls outside the sandbox.

## Context / Why

## Acceptance criteria

- [ ] Tiber MCP create/update/prioritize operations can be configured to execute outside the Codex sandbox without requiring the user to rerun equivalent Tiber CLI commands manually.
- [ ] Initialization documents and prefers a narrow allowlist for Tiber's signed Git write/sync operations, and does not recommend running the whole Tiber MCP server outside the sandbox unless the narrow permission is insufficient.

## Subtasks

## Notes / Log
