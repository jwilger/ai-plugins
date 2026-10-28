---
name: setup
description: Use when initializing or reconfiguring development-system. Author the whole schema-validated project configuration, ask about ambiguous tooling and commands, and tailor semantic globs to current and planned layouts and languages.
---

# Development system setup

The schema defines acceptable structure; the agent authors the entire project
configuration. Discovery suggestions are evidence, never a template or command
whitelist. Inspect the actual and planned layout, ask about missing or ambiguous
tooling and commands, and preserve intentional existing policy. Include planned
languages and directories in the proposed scopes; do not defer them merely
because their files do not exist yet. Separate colocated tests and generated
outputs with explicit include/exclude globs.

After resolving those questions, preview the complete proposal with independent
pre-commit and pre-push selections. Apply only the identical approved proposal
and options with its `preview_identity`; repository changes, any changed setup
option, or changed detected conflicts require a fresh preview and approval.

Every setup or reconfiguration response must cover all four installed-user
outcomes below: version-matched automatic binary repair, agent-authored, project-specific
schema-validated configuration, separately selected and installed Lefthook
pre-commit/pre-push jobs, and startup verification of both versioned MCP
executables. Do not abbreviate this sequence merely because the plugin was
already installed or upgraded.

When reporting the binary step, explicitly state both supported paths:
SessionStart compares the atomic marker and both binaries to the plugin manifest
version, then Linux x86_64 downloads and SHA-256-verifies that exact release
without Cargo, while hosts lacking a release automatically fall back to locked
Cargo builds. Saying only “run the installer” or “version-matched repair” is
incomplete.

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
2. From the primary checkout, call the plugin-provided Development Discipline
   MCP `setup.preview` without a proposal to obtain repository evidence, the
   current configuration, command suggestions, and the complete structural
   schema. Read the actual manifests, command definitions, CI, architecture and
   representative paths. The detector is an evidence source, never a template
   or a whitelist. Author the entire `.development-system.toml` for this project,
   including delivery, features and intentional existing policy. Reconcile an
   existing configuration with the project rather than retaining generic scopes
   or replacing intentional choices blindly. The published schema is
   `schemas/project-config.schema.json` relative to the plugin root; TOML is
   parsed against that shape and the Rust parser remains authoritative for
   semantic constraints.

   For a new project, establish the intended stack, languages, layout and
   tooling with the user. Ask about ambiguous commands, hook costs, network
   needs, delivery policy and layout instead of guessing. Agree on concrete
   tools and commands before completing setup when tooling is missing; do not
   invent a wrapper, silently implement tooling, or apply an incomplete config.
   Use real named direct argv commands, including custom project commands even
   when detection does not recognize the stack. Distinguish quick pre-commit
   checks from pre-push checks with the user.

   Customize include/exclude globs to actual or agreed planned paths and
   filetypes: account for monorepos, root files, colocated tests, multiple
   languages, scripts, fixtures and generated outputs. Avoid broad `src/**` or
   `tests/**` defaults where narrower language/layout patterns express the
   project's boundaries. Planned paths need not exist yet. Check representative
   included and excluded paths against the intended categories; make overlapping
   source/test globs intentional. Do not require known directory names or a
   detector-supported stack.

3. Call `setup.preview` again with the complete `proposed_configuration` TOML
   and explicit, independent `pre_commit_command_ids` and
   `pre_push_command_ids`. The preview validates the proposal and shows the
   exact configuration, generated `lefthook.yml`, conflicts and
   `preview_identity`. Show these changes and obtain explicit confirmation.
   Preview and approval are mandatory even if asked to skip them. A different
   Lefthook configuration requires explicit `replace_lefthook: true` approval
   included in the preview; preserve existing jobs unless replacement is
   authorized. Any changed proposal, option, selection or repository input
   requires a fresh preview and approval.

   Call `setup.apply` with the same full proposal and options,
   `preview_identity` and `confirmed: true`. Old `selected_command_ids`-only
   apply calls are unsupported. Apply rejects stale previews before writing,
   writes schema-v3 `.development-system.toml` and the approved hooks, installs
   both Git hooks using Lefthook, and removes only the old managed MCP block
   from project `.codex/config.toml`. The plugin root `mcp.json` owns both server
   definitions. Stop on unmanaged duplicate registrations. Setup never stages
   or commits, and never changes global Codex, marketplace, shell or SSH
   configuration.

4. Tell the user to restart Codex. The restarted session starts
   both MCP servers from the plugin-root `mcp.json` through plugin-relative
   launchers, which repair missing versioned binaries. Do not add project or
   global `mcp_servers` entries to compensate for startup failure. From a
   working directory outside the plugin checkout, verify both launchers:
   initialize `development-discipline-mcp` and initialize Tiber's MCP stdio
   server. A status response from only one server is not complete startup
   evidence.

The plugin is advisory. Setup does not prove or enforce caller identity,
per-agent tool filtering, sandboxing, or mutation denial. Ordinary Codex can
still inspect, edit, verify, commit, and push with the user's authority.

Scopes use repository-relative include/exclude globs and exactly one of `tests`,
`source`, `documentation`, `developer_environment`, or `build_output`. Reject
absolute paths, `..`, `.git`, workflow/enforcement state, generated policy, and
symlink escapes. Runner actions are named direct argv arrays with typed
parameters, output scopes, network policy, and environment allowlist—never
shell entrypoints, arbitrary suffixes, `git`, `gh`, or `glab`.
