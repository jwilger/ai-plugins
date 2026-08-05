# Development System

The single personal development plugin for Codex and Claude Code.

It defaults to direct-to-trunk delivery with Tiber and on-demand linked
worktrees for concurrent mutable work. One
project file, `.development-system.toml`, selects optional agentic-system and
eval-reporting capabilities.

Run setup from the primary checkout:

```shell
development-system setup --project . --preset personal-trunk --dry-run
development-system setup --project . --preset personal-trunk --apply --yes
```

Setup refuses linked worktrees, previews before mutation, requires explicit
confirmation, and creates exactly one initialization commit. Existing projects
are configured from scratch; there is no legacy migration path.

Use `--delivery` plus repeatable `--enable` and `--disable` options to select
Tiber, agentic-system guidance, and eval reporting. Worktree support is always
available and does not reserve the primary checkout.

`development-system doctor --project .` reports conflicting plugins, disabled
hook settings, managed-only hook policies, and user-managed MCP configuration.
The SessionStart hook runs the same check automatically in both harnesses.

The strong recommendation is to install only this plugin. Third-party plugin
marketplaces add unnecessary supply-chain exposure. The plugin owns its bundled
MCP integrations; user-added MCPs are warned about for compatibility review,
not automatically rejected.

Skill descriptions are narrow routing indexes. Detailed workflow context is
loaded only after a matching skill routes.
