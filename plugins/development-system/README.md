# Development System

The single personal development plugin for Codex and Claude Code.

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

The editor, runner, repository, and diagnostic services are retained as
unexposed reusable components for the standalone Tiber harness. Ordinary Codex
remains able to inspect, edit, verify, commit, and push while Tiber is being
bootstrapped. Tiber, rather than this plugin, will own authoritative identity,
isolation, workflow, memory, verification, and delivery.

Codex MCP launchers resolve relative to the installed plugin root and must work
from an arbitrary caller directory. A global `[mcp_servers.*]` compatibility
override is neither required nor part of supported setup.

The Codex manifest explicitly forwards the owner's `SSH_AUTH_SOCK` to both
trusted installed-root MCP launchers so their EventCore commits remain signed.
If a signed append reports that the signing agent is unavailable, upgrade or
reinstall the plugin before restarting Codex; no signing fallback is supported.

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
