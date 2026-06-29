# AGENTS.md

Guidance for AI agents (Claude Code, Codex, etc.) working in this repository.

## What this repo is

`ai-plugins` is a **multi-harness AI plugin marketplace**. It currently
implements the [Claude Code marketplace](https://code.claude.com/docs/en/plugin-marketplaces)
format and is structured to also serve Codex and other harnesses as they adopt
the plugin concept.

- The marketplace manifest is [`.claude-plugin/marketplace.json`](.claude-plugin/marketplace.json).
- Each plugin is a subdirectory of [`plugins/`](plugins/).
- The user-facing catalog lives in [`README.md`](README.md), grouped by harness.

## Development environment

Use the Nix devshell — do not install global toolchains by hand.

```shell
nix develop                       # provides node, npm, cargo, jq, prettier, rg, fd
```

**Critical convention:** anything a package manager would normally install
"globally" must land in the git-ignored `./.dependencies/` directory, not in
`$HOME`. The devshell enforces this by setting `NPM_CONFIG_PREFIX`,
`NPM_CONFIG_CACHE`, and `CARGO_HOME` to point inside `./.dependencies/` and
prepending their `bin/` dirs to `PATH`. So:

- `npm install -g <pkg>` → installs to `./.dependencies/npm/`
- `cargo install <crate>` → installs to `./.dependencies/cargo/`

Never commit `./.dependencies/`. If the environment looks broken, `rm -rf
.dependencies` and re-enter the devshell.

`.envrc` (`use flake`) is git-ignored here per the maintainer's global config;
recreate it locally if you use direnv.

## Adding a plugin

1. Create `plugins/<plugin-name>/` (kebab-case, no spaces — the name is
   public-facing and used for namespacing, e.g. `/<plugin-name>:<skill>`).
2. Add `plugins/<plugin-name>/.claude-plugin/plugin.json`. Only `name` is
   strictly required; prefer also setting `description`, `version` (semver),
   `author`, and `license`.
3. Put components at the **plugin root** (NOT inside `.claude-plugin/`):
   - `skills/<name>/SKILL.md` — adds to defaults; the primary mechanism for new work.
   - `agents/<name>.md` — subagents.
   - `commands/<name>.md` — legacy flat-file slash commands (prefer `skills/`).
   - `hooks/hooks.json`, `.mcp.json`, `.lsp.json`, `bin/` — as needed.
4. Register the plugin in `.claude-plugin/marketplace.json` by appending to the
   `plugins` array. Because `metadata.pluginRoot` is `./plugins`, `source` is
   just the directory name:
   ```json
   {
     "name": "<plugin-name>",
     "source": "<plugin-name>",
     "description": "…",
     "version": "0.1.0",
     "keywords": ["…"],
     "category": "…"
   }
   ```
5. Add a row to the matching harness table in `README.md`.
6. Give the plugin its own `README.md` stating what it does and which
   harness(es) it targets.

## Validation (do this before claiming completion)

```shell
jq empty .claude-plugin/marketplace.json          # manifest is valid JSON
find plugins -name plugin.json -exec jq empty {} \;  # every plugin manifest valid
prettier --check "**/*.{json,md}"                 # formatting (use --write to fix)
```

For an end-to-end check in Claude Code: `/plugin marketplace add .` then
`/plugin install <plugin-name>@ai-plugins`.

## Conventions

- **Names** are kebab-case, no spaces (marketplace `name`, plugin `name`,
  skill/agent directory and file names).
- **JSON** is 2-space indented; run `prettier --write` on changed `.json`/`.md`.
- **Only `.claude-plugin/`** lives inside the `.claude-plugin/` folder. All
  component directories (`skills/`, `agents/`, …) live at the plugin root.
- **Versioning:** set `version` in `plugin.json` to pin a release; if omitted,
  the git commit SHA is used as the version.

## Multi-harness notes

- Claude Code reads `.claude-plugin/marketplace.json` and per-plugin
  `.claude-plugin/plugin.json`. Keep these the source of truth.
- When adding Codex (or other-harness) support, do not break the Claude Code
  manifest. Prefer additive, harness-namespaced metadata and a parallel
  manifest if a harness needs a different format, rather than overloading
  `marketplace.json`. Always note a plugin's supported harnesses in its README
  and the `README.md` catalog tables.

## Reference

- Marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Plugin reference (full `plugin.json` schema): https://code.claude.com/docs/en/plugins-reference
- Creating plugins: https://code.claude.com/docs/en/plugins
- Discover & install: https://code.claude.com/docs/en/discover-plugins
