---
title: Build writable downstream code-quality benchmarks for Codex plugins
blocked_by: []
blocks: []
tags: [codex, evals, quality, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Measure whether the marketplace makes Codex produce better code in disposable downstream repositories, comparing no plugins, targeted quality-core plugins, and the full marketplace.

## Context / Why

Current behavior evals mostly score read-only advice and do not establish implementation-quality lift. Build realistic writable feature, bugfix, and refactor scenarios with public-surface verifiers. Start with a real personal project when suitable; otherwise use a Rust CLI plus one TypeScript or Python service. Exact targeted-plugin composition from ticket hgyz is a prerequisite. Keep fixtures scrubbed and disposable.

## Acceptance criteria

- [ ] The benchmark runs representative writable feature, bugfix, and refactor scenarios in disposable downstream repositories.

## Subtasks

## Notes / Log
