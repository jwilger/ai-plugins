---
title: Update adm-zip to prevent memory exhaustion from unsafe ZIP files
blocked_by: []
blocks: []
tags: [security, dependencies, promptfoo, dependabot]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Remove the vulnerable adm-zip version reported by GitHub Dependabot alert 2 while keeping the Promptfoo evaluation toolchain working.

## Context / Why

adm-zip is a transitive development dependency brought in through Promptfoo, Hugging Face Transformers, and onnxruntime-node. Versions below 0.6.0 can allocate roughly 4 GB of memory from a tiny crafted ZIP file and crash the process. This repository does not currently accept untrusted ZIP uploads, so the immediate exposure is limited, but the vulnerable lockfile should still be removed. Prefer upgrading the owning dependency chain; use a package override only if compatibility and maintenance behavior are verified.

## Acceptance criteria

- [ ] package-lock.json resolves adm-zip to version 0.6.0 or newer.
- [ ] The full repository CI gate and Promptfoo evaluation dry-run pass with the updated dependency tree.

## Subtasks

## Notes / Log
