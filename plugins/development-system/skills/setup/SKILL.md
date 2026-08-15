---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including semantic path scopes and named direct-argv commands.
---

# Development system setup

1. Resolve the installed Development System plugin root relative to this skill,
   then run `scripts/install-development-system-binaries.sh` from that root.
   This is a required setup prerequisite: it builds and installs the current
   host's `tiber` and `development-discipline-mcp` executables with Cargo. Do
   not use Nix as a requirement and do not defer this to the user. If Cargo or
   its required dependencies are unavailable, stop with the installer's
   diagnostic; do not configure a repository whose MCPs cannot start.
2. From the primary checkout, call `setup.preview`. Select at least one detected
   verification/test command, preferring `recommended_command_ids`; an empty
   catalog cannot configure the workflow. Show the exact preview and ask for explicit
   confirmation. Preview and confirmation are mandatory even if asked to skip
   them. Any changed option, conflict, scope, or command selection requires a
   fresh preview and approval.
3. Call `setup.apply` with `confirmed: true`. It writes schema-v3
   `.development-system.toml` in the primary checkout and never stages or
   commits. It generates no privileged profiles or agents, boundary proofs,
   activation files, or repository launch wrappers. It does not modify global
   Codex, Claude, marketplace, MCP, shell, or SSH configuration.
4. From a non-plugin working directory, verify that both installed host-local MCP servers
   start through launchers resolved from the installed plugin root. Do not add a
   global `mcp_servers` override to compensate for startup failure.

The plugin is advisory. Setup does not prove or enforce caller identity,
per-agent tool filtering, sandboxing, or mutation denial. Ordinary Codex can
still inspect, edit, verify, commit, and push with the user's authority;
standalone Tiber owns authoritative workflow and isolation.

Scopes use repository-relative include/exclude globs and exactly one of `tests`,
`source`, `documentation`, `developer_environment`, or `build_output`. Reject
absolute paths, `..`, `.git`, workflow/enforcement state, generated policy, and
symlink escapes. Runner actions are named direct argv arrays with typed
parameters, output scopes, network policy, and environment allowlist—never
shell entrypoints, arbitrary suffixes, `git`, `gh`, or `glab`.
