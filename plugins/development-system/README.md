# Development System

The single personal development plugin for Codex and Claude Code.

## Host-local binaries

After installing or upgrading this plugin, build its Rust executables once from
the marketplace checkout:

```shell
just install-development-system-binaries
```

The command builds only the current host and atomically installs `tiber` and
`development-discipline-mcp` in
`$XDG_DATA_HOME/ai-plugins/development-system/<plugin-version>/<host>/`.
When `XDG_DATA_HOME` is unset, it uses `~/.local/share`. Re-running the command
is safe; a newer plugin version is installed alongside previous versions.
Project-local MCP configuration always uses these stable host-directory paths,
so a same-version reinstall can atomically replace its staging target without
leaving a project pointed at it.
The published plugin intentionally carries no bootstrap MCP manifest. During
project setup, it writes only the current harness's project-local MCP
configuration—`.codex/config.toml` for Codex or `.mcp.json` for Claude Code.
Those entries launch the installed binaries by their absolute XDG paths; they
do not launch a plugin-relative shell wrapper. Re-run setup after an upgrade,
then start a new harness session.
The bootstrap requires a working Cargo environment; the repository's Nix
devshell is optional. If `just` is unavailable, run
`scripts/install-development-system-binaries.sh` from the marketplace checkout.

The `setup` skill runs this bootstrap automatically before it configures a
repository, so manual use is only needed when you want the MCPs available
before running setup.

It is inert outside a Git repository or without a valid schema-3
`.development-system.toml`. Read-only repository inspection remains available
in every state. Start configuration through the structured `setup.preview`
tool, review the discovered path scopes and named command catalog, then
explicitly confirm `setup.apply`. Setup never stages or commits configuration.

The plugin-wide Development Discipline MCP surface provides bounded repository
inspection, deterministic setup, and multi-agent final review. Those services
and the Codex hooks are advisory: they
do not establish agent identity, isolate project tools, execute project
mutations, or deny ordinary host capabilities. `setup.apply` writes only
repository-local configuration. It does not generate privileged agents or
profiles and never changes global Codex, Claude, marketplace, MCP, shell, or
SSH settings.

Within that advisory boundary, final-review state fails closed: risk planning
must select a lens, every selected lens and assigned verifier reruns in each
iteration, and completion requires at least three consecutive complete
finding-free iterations. Findings, malformed results, and material deltas reset
the streak; the review-budget `ship` choice cannot bypass it. The planned
standalone Tiber scheduler is the authority layer that can eventually make
those receipts prerequisites for repository mutation. That scheduler binding
is not part of the currently installed Tiber task-board binary.

The editor, runner, repository, and diagnostic services are retained as
unexposed reusable components for the standalone Tiber harness. Ordinary Codex
remains able to inspect, edit, verify, commit, and push while Tiber is being
bootstrapped. Tiber, rather than this plugin, will own authoritative identity,
isolation, workflow, memory, verification, and delivery.

Codex and Claude Code each consume project-local direct binary entries. A global
`[mcp_servers.*]` compatibility override is neither required nor part of
supported setup.

The project-local MCP entries use the harness's ordinary process environment.
If a signed append reports that the signing agent is unavailable, upgrade or
reinstall the plugin before restarting the harness; no signing fallback is
supported.

The strong recommendation is to install only this plugin. Third-party plugin
marketplaces add unnecessary supply-chain exposure. The plugin owns its bundled
MCP integrations; user-added MCPs are warned about for compatibility review,
not automatically rejected.

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
