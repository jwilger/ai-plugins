---
title: Fix Codex MCP startup failures for tiber and promptfoo
blocked_by: []
blocks: []
tags: []
---

## Summary

Codex startup still reports MCP client failures for tiber and promptfoo: MCP startup failed with No such file or directory (os error 2), followed by MCP startup incomplete. A previous fix was thought to resolve this, but the error remains visible when launching Codex in the ai-plugins checkout. Diagnose the exact command/path/env mismatch and fix it at the source.

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log
