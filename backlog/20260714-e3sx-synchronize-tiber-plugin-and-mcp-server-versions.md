---
title: Synchronize Tiber plugin and MCP server versions
blocked_by: []
blocks: []
tags: [minor, tiber, release, versioning, mcp]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Keep Tiber's published plugin version, Rust workspace packages, bundled binaries, checksums, and MCP initialize serverInfo.version synchronized.

## Context / Why

Verified MINOR release finding from 20260709-spx8: plugin manifests/launcher publish 0.9.0 while the Rust workspace, Cargo.lock, and shipped MCP binary still report 0.8.0.

## Acceptance criteria

- [ ] Installing the published Tiber version yields the same semver from both plugin manifests and MCP initialize on every bundled platform, with refreshed package locks, binaries, and checksums.

## Subtasks

## Notes / Log

- 2026-07-14: 2026-07-14 formal final-review pass 1 for 20260709-spx8 reconfirmed that Tiber plugin 0.9.0 packages an MCP runtime reporting 0.8.0. Deferred as MINOR; covered by this ticket's existing runtime/package synchronization criterion.
