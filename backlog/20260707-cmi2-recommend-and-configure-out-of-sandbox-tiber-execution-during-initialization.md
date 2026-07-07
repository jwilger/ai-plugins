---
title: Recommend and configure out-of-sandbox Tiber execution during initialization
blocked_by: []
blocks: []
tags: []
---

## Summary

Tiber initialization should detect when MCP-backed task writes are likely to fail inside the Codex sandbox, recommend configuring Tiber MCP calls to execute outside the sandbox, and offer to set up that permission/workflow for the user so MCP task operations do not require manual CLI fallback.

## Context / Why

## Acceptance criteria

- [ ] Tiber MCP create/update/prioritize operations can be configured to execute outside the Codex sandbox without requiring the user to rerun equivalent Tiber CLI commands manually.

## Subtasks

## Notes / Log
