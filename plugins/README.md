# `plugins/`

Each subdirectory here is one plugin. The marketplace manifests point to plugin
directories explicitly: Claude Code entries use `./plugins/<plugin-name>`, and
Codex entries use `{ "source": "local", "path": "./plugins/<plugin-name>" }`.

## Anatomy of a plugin

```
plugins/my-plugin/
├── .claude-plugin/
│   └── plugin.json        # manifest — only `name` is strictly required
├── skills/                # <name>/SKILL.md  → /my-plugin:<name>
├── commands/              # legacy flat-file slash commands (prefer skills/)
├── agents/                # <name>.md         → subagent /my-plugin:<name>
├── hooks/hooks.json       # event hooks
├── .mcp.json              # MCP server definitions
└── README.md              # what it does, which harnesses it targets
```

Only the `.claude-plugin/` folder is special; every other component directory
lives at the plugin root. See [`../AGENTS.md`](../AGENTS.md) for the full
authoring workflow and validation steps.
