---
title: Make Tiber MCP work with Codex SSH signing boundaries
blocked_by: []
blocks: []
tags: [tiber, mcp, codex, ssh, signing, nixos]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
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
- 2026-07-09: Codex config experiment for SSH signing boundary: adding `[plugins."tiber@ai-plugins".mcp_servers.tiber] env_vars = ["SSH_AUTH_SOCK"]` alone was tested after restart and did not resolve the Tiber MCP write failure; `git commit-tree -S` still reported `Couldn't get agent socket?`. A broader permission-profile attempt to allow `/home/jwilger/.1password/agent.sock` as a Unix socket initially broke shell tool execution because the profile omitted Codex runtime paths, so plugin initialization should not blindly activate a custom profile. Next investigation should determine the minimal safe permission-profile shape and whether plugin-provided MCP server config can reliably request or guide SSH agent socket forwarding/allowlisting.
