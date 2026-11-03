# `plugins/`

Each subdirectory here is one plugin. Codex marketplace entries point to plugin
directories with `{ "source": "local", "path": "./plugins/<plugin-name>" }`.

## Anatomy of a plugin

```
plugins/my-plugin/
├── plugin.json           # Agent Plugins manifest
├── mcp.json              # optional plugin-owned MCP servers
├── skills/                # <name>/SKILL.md  → /my-plugin:<name>
├── commands/              # legacy flat-file slash commands (prefer skills/)
├── agents/                # <name>.toml       → subagent /my-plugin:<name>
├── hooks/codex.json       # Codex event hooks
└── README.md              # what it does
```

Component directories live at the plugin root. See
[`../AGENTS.md`](../AGENTS.md) for the full authoring workflow and validation
steps.
