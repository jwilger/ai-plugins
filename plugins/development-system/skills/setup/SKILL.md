---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including semantic path scopes and named direct-argv commands.
---

# Development system setup

1. Resolve the installed Development System plugin root relative to this skill,
   then run `scripts/install-development-system-binaries.sh --auto` from that
   root. The installed `SessionStart` hook performs the same check on every
   session: both executables and their atomic installation marker must match the
   plugin manifest version. Missing or stale installations are repaired. Linux
   x86_64 downloads the exact-version GitHub Release bundle, verifies its
   SHA-256 checksum and archive layout, and atomically installs `tiber` and
   `development-discipline-mcp` without Cargo or Nix. A host without a prebuilt
   release automatically performs both locked Cargo builds and therefore needs
   a working Cargo environment. Do not defer either path to the user. If
   download, verification, extraction, or source compilation fails, stop with
   the installer's diagnostic; do not configure a repository whose MCPs cannot
   start.
2. From the primary checkout, invoke the installed `development-discipline-mcp`
   binary directly for `setup.preview`, then `setup.apply`. The preview detects
   manifests, lockfiles, source/test/documentation directories, build outputs,
   the repository's Nix wrapper, and stack-native test runners; it emits only
   project-supported scopes and named direct-argv command candidates. Do not
   replace that discovery with a generic template. Select at least one detected
   verification/test command, preferring `recommended_command_ids`; an empty
   command catalog or a repository with no detectable scope cannot configure
   the workflow. Show the exact schema-validated preview and ask for explicit
   confirmation. Preview and confirmation are mandatory even if asked to skip
   them. Any changed option, conflict, scope, or command selection requires a
   fresh preview and approval.
3. Call `setup.apply` with `confirmed: true`. It writes
   schema-v3 `.development-system.toml` plus only the owned local MCP settings:
   `.codex/config.toml` for Codex. It never stages or commits, and never changes
   global Codex, marketplace, shell, or SSH
   configuration.
4. Tell the user to restart Codex. The restarted session starts
   both MCP servers from the concrete absolute XDG binary paths in that project's
   configuration. Do not add a global `mcp_servers` override to compensate for
   startup failure. From a working directory outside the plugin checkout,
   verify both installed executables through those versioned absolute paths:
   initialize `development-discipline-mcp` and initialize Tiber's MCP stdio
   server. A status response from only one server is not complete startup
   evidence.

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
