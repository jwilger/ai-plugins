# Development System

The single personal development plugin for Codex and Claude Code.

It is inert outside a Git repository or without a valid schema-3
`.development-system.toml`. Read-only repository inspection remains available
in every state. Start configuration through the structured `setup.preview`
tool, review the discovered path scopes and named command catalog, then
explicitly confirm `setup.apply`. Setup never stages or commits configuration.

The plugin-wide Development Discipline MCP surface is deliberately limited to
repository inspection and deterministic setup. Its workflow guidance and Codex
hooks are advisory: they do not establish agent identity, isolate tools, or
deny ordinary host capabilities. `setup.apply` writes only repository-local
configuration. It does not generate privileged agents or profiles and never
changes global Codex, Claude, marketplace, MCP, shell, or SSH settings.

The semantic workflow, editor, runner, repository, and diagnostic services are
retained as reusable components for the standalone Tiber harness. Ordinary
Codex remains able to inspect, edit, verify, commit, and push while Tiber is
being bootstrapped. Tiber, rather than this plugin, will own authoritative
identity, isolation, workflow, memory, verification, and delivery.

Codex MCP launchers resolve the versioned installed plugin root and must work
from an arbitrary caller directory. A global `[mcp_servers.*]` compatibility
override is neither required nor part of supported setup.

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

The top-level plugin manifest starts the read/setup-only Development Discipline
surface and the independent Tiber MCP server. Development Discipline stores
local final-review events in SQLite and uses `development-workflow` only for
its workflow authority; it reads Tiber's unresolved CI hold when gating
delivery. Tiber is the sole CI-incident and receipt authority on `tiber`, in
addition to publishing task events there.
Promptfoo remains an optional MCP owned by the
agentic-systems-engineering component and is intentionally excluded from the
top-level manifest because projects may disable that capability and must supply
the pinned Promptfoo runtime separately. Configure it explicitly when needed;
do not install the retained component as a separate marketplace plugin.

Skill descriptions are narrow routing indexes. Detailed workflow context is
loaded only after a matching skill routes.
