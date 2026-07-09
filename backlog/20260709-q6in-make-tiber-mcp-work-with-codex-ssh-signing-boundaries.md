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
- [ ] Implement or document a narrow recovery path for NixOS/Home Manager SSH config symlink ownership, 1Password SSH signing, and Codex sandbox Git writes.
- [ ] Tiber MCP can create and sync a backlog task from Codex with signed commits on refs/heads/tasks.
- [ ] Avoid broad raw Git approvals, whole-MCP-server unsandboxing, forced private-key ssh -i workarounds, and manual task-branch commits as the normal path.
- [ ] Add regression coverage or a smoke test where practical.
- [ ] Docs explain how agents distinguish sandbox permission failures from host SSH-agent or SSH-config inheritance failures.

## Subtasks

## Notes / Log

- 2026-07-09: Known nuclear workaround: adding a Codex approval rule that allows all `git` commands to run unsandboxed appears to make this class of Tiber MCP task-branch operations work. Treat this as evidence for the failure boundary and as a last-resort diagnostic/workaround, not as the desired normal path; the ticket should still aim for a narrower Tiber-specific/sandbox-aware recovery path.
