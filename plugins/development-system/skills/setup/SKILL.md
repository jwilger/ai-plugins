---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including semantic path scopes, named direct-argv commands, generated harness profiles, and boundary-proof status.
---

# Development system setup

1. From the primary checkout, call `setup.preview`. Select at least one detected
   verification/test command, preferring `recommended_command_ids`; an empty
   catalog cannot activate mutation. Show the exact preview and ask for explicit
   confirmation. Preview and confirmation are mandatory even if asked to skip
   them. Any changed option, conflict, scope, or command selection requires a
   fresh preview and approval.
2. Call `setup.apply` with `confirmed: true`. It writes schema-v3 repository
   configuration and fail-closed profiles, never stages or commits. If Codex's
   read-only MCP filesystem causes `development_system.setup_policy_write_failed`,
   run only its byte-for-byte `recovery_command`; never reconstruct or extend it.
3. For Codex mutation, call `setup.probe` with `harness: "codex"` and
   `confirmed: true`. It must use an authenticated Codex CLI in both a disposable
   Git project and disposable `CODEX_HOME`, proving caller identity, root
   mutation denial, per-agent MCP filtering, and named-agent service isolation.
   Never synthesize its short-lived, version/configuration-bound receipt.
4. Re-run confirmed `setup.apply`. Proof alone grants nothing; this apply binds
   activation to the proof and exact generated-profile digest. Start a new Codex
   session—capabilities do not appear dynamically in the current one.
5. In that session prove root built-in mutation denial; give a current assignment
   to a concrete generated agent such as `development-system-implementer` or
   `development-system-verifier`; verify it reaches only its role services; and
   require its `project-runner` to return a durable receipt for the selected
   command. From a non-plugin working directory, also prove packaged MCP startup
   through manifest-relative launchers anchored to the installed plugin root,
   without global `mcp_servers` overrides.

Keep mutation unavailable when commands, proof, activation, or profiles are
absent, stale, version/configuration-mismatched, or tampered. Codex is the
supported privileged harness; Claude remains read-only until it proves
equivalent identity and per-agent containment.

Scopes use repository-relative include/exclude globs and exactly one of `tests`,
`source`, `documentation`, `developer_environment`, or `build_output`. Reject
absolute paths, `..`, `.git`, workflow/enforcement state, generated policy, and
symlink escapes. Runner actions are named direct argv arrays with typed
parameters, output scopes, network policy, and environment allowlist—never
shell entrypoints, arbitrary suffixes, `git`, `gh`, or `glab`.
