---
title: Migrate CI off Nix devshell tooling
blocked_by: []
blocks: []
tags: [ci, github-actions, developer-experience, tooling]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Replace Nix realization inside GitHub Actions with pinned standard hosted-runner setup while preserving the repository's existing CI semantics and keeping Nix/EMC integration explicitly local-only.

## Context / Why

All substantive GitHub CI jobs currently install Nix and run through nix develop, even though EMC and flake integration are local development concerns. The migration must inventory every command behind just ci and install its real prerequisites on a clean hosted runner, including the repository-pinned Rust toolchain, Node, jq, Prettier, Bats, actionlint/yq where used, cargo-mutants, and any browser or package dependencies. Preserve the aggregate CI gate, dry-run eval wiring, manifest validation, and current logical checks rather than silently dropping gates.

## Acceptance criteria

- [ ] GitHub Actions CI does not invoke Nix or realize flake devshell outputs.
- [ ] CI installs and runs the Rust, Node, formatting, and Bats tooling with standard GitHub-hosted setup.
- [ ] EMC and flake integration checks remain available as explicit local-only commands and are excluded from CI.
- [ ] The workflow explicitly installs every prerequisite exercised by just ci using pinned or repository-controlled versions, including Node and the repository Rust toolchain.
- [ ] All existing logical gates, dry-run eval checks, and the aggregate CI gate remain present and pass on a clean GitHub-hosted runner.
- [ ] Tests or workflow assertions prove CI contains no Nix installation action, nix develop invocation, or accidental EMC/flake gate.

## Subtasks

## Notes / Log
