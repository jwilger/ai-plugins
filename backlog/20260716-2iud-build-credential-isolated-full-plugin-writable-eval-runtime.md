---
title: Build credential-isolated full-plugin writable eval runtime
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Create a hostile-capable execution boundary for writable evals that can safely load full plugins, including MCP servers, hooks, and binaries, without exposing maintainer credentials or host state.

## Context / Why

Deferred from the skills-first Rust pilot. True full-plugin execution requires a credential broker or disposable VM/container boundary, explicit egress controls, process-wide resource limits, host-read denial, raw-transcript quarantine, and adversarial containment canaries. It must not reuse long-lived plaintext Codex OAuth files inside a model-readable filesystem.

## Acceptance criteria

- [ ] Codex inference authenticates through a revocable credential boundary that model-invoked commands and plugin subprocesses cannot read or replay.

## Subtasks

## Notes / Log
