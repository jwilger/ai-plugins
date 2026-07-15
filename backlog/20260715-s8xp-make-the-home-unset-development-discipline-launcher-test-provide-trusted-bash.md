---
title: Make the HOME-unset development-discipline launcher test provide trusted Bash
blocked_by: []
blocks: []
tags: [development-discipline, tests, nix, hermeticity, ci, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Keep the HOME-unset Cargo-fallback startup test focused on missing-Cargo behavior by providing the Nix devshell’s trusted Bash on its scrubbed PATH.

## Context / Why

The canonical `just ci` gate consistently fails `development-discipline MCP Cargo fallback handles unset HOME` with status 127 and `env: 'bash': No such file or directory`. The fixture invokes the Bash launcher under `env -i PATH=/bin:/usr/bin`; on NixOS neither path contains Bash, so the shebang fails before the launcher can exercise its intended HOME-unset/missing-Cargo branch. This failure is unchanged by and blocks final verification of the deterministic release-parity ticket.

## Acceptance criteria

## Subtasks

## Notes / Log
