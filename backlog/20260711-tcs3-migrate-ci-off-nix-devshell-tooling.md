---
title: Migrate CI off Nix devshell tooling
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Replace Nix-based GitHub Actions setup with standard GitHub-hosted setup for Rust, Node, and test tooling. Keep flake/EMC checks local-only.

## Context / Why

The current CI jobs run through `nix develop`, which realizes the project devshell. EMC is a local development integration and should not be built, run, or tested in CI. Rust binaries should use conventional GitHub Actions Rust setup instead.

## Acceptance criteria

- [ ] GitHub Actions CI does not invoke Nix or realize flake devshell outputs.
- [ ] CI installs and runs the Rust, Node, formatting, and Bats tooling with standard GitHub-hosted setup.
- [ ] EMC and flake integration checks remain available as explicit local-only commands and are excluded from CI.

## Subtasks

## Notes / Log
