---
title: Make Tiber MCP work with Codex SSH signing boundaries
blocked_by: []
blocks: []
tags: [tiber, mcp, codex, ssh, signing, nixos]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make normal Tiber MCP task capture work from Codex on this NixOS/Home Manager setup with 1Password SSH signing, without relying on broad unsandboxed CLI execution or manual task-branch commits.

## Context / Why

Tiber MCP and CLI writes exposed SSH config, agent socket, Git fetch, and signed-commit failures from Codex. The user shell could sign successfully, while Codex-launched commands previously inherited an SSH agent switcher socket and, when launched remotely, lacked the normal desktop/1Password environment. A reboot and locally-started Codex fixed the agent environment, but MCP writes still need a clean sandbox-aware path for Git object/ref writes and signing.

## Acceptance criteria

- [ ] Diagnose and document how Codex-launched Tiber MCP can inherit SSH and 1Password agent state differently from the user interactive shell.

## Subtasks

## Notes / Log
