---
title: Fix Codex MCP startup failures for tiber and promptfoo
blocked_by: []
blocks: [20260707-hqzv-encode-mcp-bootstrap-fix-in-marketplace-plugins]
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

Codex startup still reports MCP client failures for tiber and promptfoo: MCP startup failed with No such file or directory (os error 2), followed by MCP startup incomplete. A previous fix was thought to resolve this, but the error remains visible when launching Codex in the ai-plugins checkout. Diagnose the exact command/path/env mismatch and fix it at the source.

## Context / Why

## Acceptance criteria

- [ ] Launching Codex in the ai-plugins checkout no longer prints MCP startup failures for tiber or promptfoo.
- [ ] A regression check covers the startup path that failed with os error 2, or the task documents why such a check cannot be automated.

## Subtasks

## Notes / Log

- 2026-07-07: Implemented locally on branch mcp-startup-direct-launchers at commit e064044. Fix changes plugin MCP manifests to launch via absolute /bin/sh and adds direct manifest startup regression coverage. Verification run: nix develop -c just bats validate-marketplace; nix develop -c scripts/evals/run.sh --dry-run --suite canary.
