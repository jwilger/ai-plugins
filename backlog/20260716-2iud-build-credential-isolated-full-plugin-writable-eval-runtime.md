---
title: Safely evaluate complete plugins that can run code
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Build an isolated writable evaluation environment for full plugins, including their servers, hooks, and executable programs. Plugin code must not be able to read or replay maintainer credentials, access host data, escape resource limits, or export unreviewed artifacts.

## Context / Why

Deferred from the skills-first Rust pilot. True full-plugin execution requires a credential broker or disposable VM/container boundary, explicit egress controls, process-wide resource limits, host-read denial, raw-transcript quarantine, and adversarial containment canaries. It must not reuse long-lived plaintext Codex OAuth files inside a model-readable filesystem.

## Acceptance criteria

- [ ] Codex inference authenticates through a revocable credential boundary that model-invoked commands and plugin subprocesses cannot read or replay.
- [ ] Full plugin MCP, hook, and executable surfaces run inside a disposable filesystem, PID, resource, and network boundary with allowlisted egress.
- [ ] Containment canaries prove host-home reads, sibling writes, command-network access, credential access, process escape, and artifact exfiltration fail closed.
- [ ] Only secret-scanned allowlisted evidence leaves the disposable runtime; contaminated raw output is quarantined and never shared.
- [ ] The skills-only pilot remains the default until this boundary passes adversarial review and repeatable end-to-end tests.

## Subtasks

## Notes / Log
