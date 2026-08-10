---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including semantic path scopes, named direct-argv commands, generated harness profiles, and boundary-proof status.
---

# Development system setup

Use `setup.preview` first. It discovers repository-relative scope candidates
and previews an empty named-command catalog without writing.

Ask for confirmation, then call `setup.apply` with `confirmed: true`. It writes
only schema-v3 `.development-system.toml` and generated read-only policy. It
does not stage, commit, invoke Git, or enable mutation services.

Define scopes using repository-relative include/exclude globs and one fixed
category: `tests`, `source`, `documentation`, `developer_environment`, or
`build_output`. Never admit absolute paths, `..`, `.git`, workflow state,
enforcement configuration, generated policy, or a symlink escape.

Define every runner action as a named direct argv array with typed parameters,
declared output scopes, network policy, and environment allowlist. Do not use a
shell entrypoint, arbitrary suffix, `git`, `gh`, or `glab` as a project command.

Generated profiles stay read-only until the disposable Claude/Codex boundary
spike proves caller identity, per-agent MCP filtering, and an OS-enforced write
boundary. If proof is absent or stale, leave mutation unavailable.
