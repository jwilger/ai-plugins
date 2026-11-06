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

Every setup or reconfiguration response must cover version-matched binary
repair, agent-authored project configuration, separately selected Lefthook
pre-commit/pre-push jobs, and startup verification of the required MCP servers.
Development System owns `development-discipline`; the independently installed
Tiber plugin owns `tiber`. Default marketplace installation installs both.

1. Resolve the installed Development System plugin root relative to this skill
   and run `scripts/install-development-system-binaries.sh --auto`. Its
   SessionStart hook checks only `development-discipline-mcp` and its atomic
   version marker. Linux x86_64 downloads and SHA-256-verifies the exact release
   without Cargo; unsupported hosts use locked Cargo builds. An isolated
   installed plugin obtains a checksum-verified exact-release source bundle
   containing all transitive Rust sources rather than depending on a companion
   cache. If Tiber integration is enabled, require the installed `tiber@ai-plugins`
   companion, resolve its root through its installed skill, and run its
   `scripts/install-tiber-binaries.sh --auto`. Tiber's launcher performs its own
   version check and repair. Do not defer repair to the user or configure a
   repository whose required MCPs cannot start. A missing companion must produce
   installation guidance; disabling integration does not bypass recovery holds.
2. Establish extremely frequent small coherent gated commits for every delivery
   mode. Direct-to-trunk pushes target origin’s configured trunk; PR/MR pushes
   target the selected branch; local-only commits remain local. Honor explicit
   no-commit restrictions. Propagate this distinction into generated project
   guidance so local-only and PR projects inherit the commit cadence too.
   From the primary checkout, call the plugin-provided Development Discipline
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

   Establish delivery cadence explicitly. For direct-to-trunk, agree on one
   checkout on the configured trunk branch, local signed commits and direct
   pushes to origin's trunk, extremely frequent small coherent increments,
   configured pre-commit checks for every commit, proportional independent
   publication review, and exact-SHA CI. Do not require completion of a feature
   or vertical slice before publication. Establish regular usable-slice and
   final-MVP acceptance separately; supporting commits do not earn user-value
   credit. Explain CI failure preemption, identifiable WIP stashes preserving
   index and untracked files, conflict-safe restoration, and explicit handling
   of unpublished commits. Propagate these choices into generated AGENTS.md
   and project workflow rules using the scaffold guidance. Preserve intentional
   PR/MR and local-only choices; never silently convert their delivery mode.

   Customize include/exclude globs to actual or agreed planned paths and
   filetypes: account for monorepos, root files, colocated tests, multiple
   languages, scripts, fixtures and generated outputs. Avoid broad `src/**` or
   `tests/**` defaults where narrower language/layout patterns express the
   project's boundaries. Planned paths need not exist yet. Check representative
   included and excluded paths against the intended categories; make overlapping
   source/test globs intentional. Do not require known directory names or a
   detector-supported stack.

   Strongly recommend the [architecture and decision-history practices](../engineering-standards/references/architecture-policy.md)
   during setup. Inventory existing architecture, ADRs, review and delivery
   conventions first. Present the practices and tradeoffs as adoption choices;
   respect existing policy, including alternate paths, ADR backlinks, editable
   records or batching when the project chooses them. Do not silently migrate
   policy or treat unadopted practices as setup blockers.

   Record only the accepted subset, overrides and prospective boundary in
   project-owned guidance. Setup/configuration application and plugin installation
   do not imply adoption. For authorized scaffolding, load the retained scaffold
   templates, adapt them to the accepted choices, and generate only adopted
   requirements. No new configuration schema keys are needed for this policy.
   Offer optional checker integration for supported adopted rules; do not enable
   the full profile implicitly. Report unsupported conventions and semantic
   review needs honestly. Preserve unrelated work and historical records during
   any separately authorized migration; never reconstruct old commits.

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
   from project `.codex/config.toml`. Each installed plugin owns its server definition in its root
   `mcp.json`. Stop on unmanaged duplicate registrations. Setup never stages
   or commits, and never changes global Codex, marketplace, shell or SSH
   configuration.

4. Tell the user to restart Codex. Each installed plugin starts its own MCP
   server through its plugin-relative launcher and repairs its own versioned
   binary. Do not add project registrations to compensate for startup failure.
   For Tiber SSH authentication only, the marketplace installer or installed
   `configure-codex-mcp.py --apply` maintains a complete shared user transport
   with `env_vars = ["SSH_AUTH_SOCK"]` and a trusted absolute installed launcher.
   Follow the Tiber plugin guidance; preserve unmanaged registrations. This
   repair requires a full Codex restart from the current agent shell and never
   embeds socket values or keys. Setup itself still does not edit global config. From a working directory outside the plugin checkout,
   initialize `development-discipline-mcp`. When `features.tiber = true`, also
   initialize Tiber's MCP stdio server and require successful startup evidence
   from both. When `features.tiber = false`, Development Discipline startup is
   sufficient for setup; do not require installation or startup of Tiber.

The plugin is advisory. Setup does not prove or enforce caller identity,
per-agent tool filtering, sandboxing, or mutation denial. Ordinary Codex can
still inspect, edit, verify, commit, and push with the user's authority.

Scopes use repository-relative include/exclude globs and exactly one of `tests`,
`source`, `documentation`, `developer_environment`, or `build_output`. Reject
absolute paths, `..`, `.git`, workflow/enforcement state, generated policy, and
symlink escapes. Runner actions are named direct argv arrays with typed
parameters, output scopes, network policy, and environment allowlist—never
shell entrypoints, arbitrary suffixes, `git`, `gh`, or `glab`.
