# Development System

The single personal development plugin for Codex.

## Host-local binaries

After installing or upgrading this plugin, download and verify its Rust
executables once from the marketplace checkout:

```shell
just install-development-system-binaries
```

On Linux x86_64, the command downloads the exact plugin-version bundle from the
repository's GitHub Release, verifies its SHA-256 sidecar and fixed archive
layout, and atomically installs `tiber` and
`development-discipline-mcp` in
`$PLUGIN_DATA/ai-plugins/development-system/<plugin-version>/<host>/`
under Codex. Direct CLI use falls back to `$XDG_DATA_HOME`, then
`~/.local/share`. Re-running the command
is safe; a newer plugin version is installed alongside previous versions.
The plugin root `mcp.json` declares both servers with plugin-relative
launchers. Each launcher repairs a missing matching binary before starting, so
consuming projects do not configure MCP servers. Project setup removes only a
previously managed Development System MCP block from `.codex/config.toml`.
Signed Tiber operations use the plugin-owned
`$PLUGIN_DATA/signing-agent-socket` file. Outside a portable harness,
the launcher uses `$XDG_CONFIG_HOME/ai-plugins/development-system/` (or
`~/.config`). The root installer captures a valid `SSH_AUTH_SOCK`; update
the plugin-owned file with an absolute socket path if your agent moves.
The release bundle contains statically linked Linux x86_64 executables, the
applicable license, and the exact source tag and commit. To force a locked
source build for diagnosis:

```shell
just install-development-system-binaries --from-source
```

The repository's Nix devshell is optional. If `just` is unavailable, run
`scripts/install-development-system-binaries.sh` from the marketplace checkout.

The installed `SessionStart` hook and both MCP launchers verify the binaries
and atomic installation marker against the plugin manifest version. It repairs
missing or stale installations automatically: Linux x86_64 uses the verified
release bundle, while hosts without a prebuilt bundle use the locked Cargo
build. The `setup` skill performs the same automatic check before configuring a
repository. Manual use is only needed for diagnosis or an intentional source
build.

It is inert outside a Git repository or without a valid schema-3
`.development-system.toml`. Read-only repository inspection remains available
in every state. Start configuration through the structured `setup.preview`
tool. It detects the project's manifests, lockfiles, conventional
source/test/docs and build paths, Nix devshell, and stack-native test commands,
then produces a schema-validated project-specific configuration instead of a
generic template. Review its exact scopes and named command catalog, then
review the generated project `lefthook.yml` and its separate pre-commit and
pre-push command selections, then explicitly confirm `setup.apply`. Setup
installs both Git hooks with Lefthook but never stages or commits configuration.

Normal Development System work relies on those Git operations for repository
verification: `git commit` triggers the fast pre-commit checks and `git push`
triggers the pre-push checks. Agents must not run the same gate commands
separately unless the user explicitly requests a diagnostic run.

The plugin-wide Development Discipline MCP surface provides bounded repository
inspection, deterministic setup, and multi-agent final review. Those services
and the Codex hooks are advisory: they
do not establish agent identity, isolate project tools, execute project
mutations, or deny ordinary host capabilities. `setup.apply` writes only the
previewed repository-local configuration and installs its Git hook launchers.
It does not generate privileged agents or
profiles and never changes global Codex, marketplace, MCP, shell, or
SSH settings.

Within that advisory boundary, final-review state fails closed: risk planning
must select a lens, every selected lens and assigned verifier reruns in each
iteration, and completion requires at least three consecutive complete
finding-free iterations. Findings, malformed results, and material deltas reset
the streak; the review-budget `ship` choice cannot bypass it. The reader status
attests the connected final-review protocol, and current workflow guidance
refuses to create review state when that attestation is missing or too old, so
a newly installed skill cannot silently coordinate through a stale one-pass MCP
runtime. The planned
Tiber can additionally enforce those receipts at the task boundary when a
project opts into `[final_review].minimum_clean_reviews`: its Git-backed task
history then blocks both task completion and trailer-driven delivery until the
required clean sequence is current. This gate is available through the same
CLI and MCP surface in Codex.

The editor, runner, repository, and diagnostic services are retained as
unexposed reusable components for the standalone Tiber harness. Ordinary Codex
remains able to inspect, edit, verify, commit, and push while Tiber is being
bootstrapped. Tiber, rather than this plugin, will own authoritative identity,
isolation, workflow, memory, verification, and delivery.

Codex loads the plugin-root `mcp.json` and plugin-relative launchers. Project
MCP entries and global compatibility overrides are unnecessary. If a signed
append cannot reach the agent, update the plugin-owned signing socket path.

The core workflow requires only this plugin. The root installer offers GitHub
and CodeRabbit as optional companions. GitHub provides a connector mapping and
CodeRabbit provides skills backed by its CLI; neither requires a Development
System MCP dependency. Superpowers is omitted because its workflow overlaps.
The plugin owns its bundled MCP integrations; user-added MCPs are warned about
for compatibility review, not automatically rejected.

The plugin root is the active public surface: its manifests, hooks, launchers,
and `skills/` directory define the installed `development-system` plugin and
the `development-system:<skill>` routing names. Directories under `components/`
retain independently authored source, tests, manifests, and runtime assets that
the public plugin owns or wraps. Those component directory and manifest names
remain valid internal identities; they are not separate marketplace install
targets or public routing labels.

The top-level plugin manifest starts the advisory Development Discipline
inspection, setup, and multi-agent final-review surface plus the independent
Tiber MCP server. The advisory surface stores final-review transitions on a
separate local-only Git-backed EventCore authority and never publishes them to
`development-workflow`; the standalone workflow service retains that remote
authority. Development Discipline reads Tiber's unresolved CI hold when
evaluating delivery readiness. Tiber is the sole CI-incident and receipt
authority on `tiber`, in addition to publishing task events there.
Promptfoo remains an optional MCP owned by the
agentic-systems-engineering component and is intentionally excluded from the
top-level manifest because projects may disable that capability and must supply
the pinned Promptfoo runtime separately. Configure it explicitly when needed;
do not install the retained component as a separate marketplace plugin.

Skill descriptions are narrow routing indexes. Detailed workflow context is
loaded only after a matching skill routes.
