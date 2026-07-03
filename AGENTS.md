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
nix develop                       # provides node, npm, jq, prettier, rg, fd, just, bats
```

**Critical convention:** anything npm would normally install "globally" must
land in the git-ignored `./.dependencies/` directory, not in `$HOME`. The
devshell enforces this by setting `NPM_CONFIG_PREFIX` and `NPM_CONFIG_CACHE` to
point inside `./.dependencies/` and prepending the local npm `bin/` dir to
`PATH`. So:

- `npm install -g <pkg>` → installs to `./.dependencies/npm/`

Never commit `./.dependencies/`. If the environment looks broken, `rm -rf
.dependencies` and re-enter the devshell.

`.envrc` (`use flake`) is git-ignored here per the maintainer's global config;
recreate it locally if you use direnv.

## Worktree workflow

This repo is configured for parallel development from linked worktrees. The main
checkout is the coordination checkout; feature work should happen in worktrees
created under the ignored repo-local `.worktrees/` directory:

```shell
git worktree add .worktrees/<branch-name> -b <branch-name>
```

Install the shared hooks once from the main checkout:

```shell
just worktree-hooks
```

The installed hooks do two things:

- `pre-commit` and `pre-push` run `scripts/worktree-guard.sh`, which blocks
  commits and pushes from the main checkout while allowing linked worktrees.
- `post-checkout` runs `scripts/worktree-bootstrap.sh`, which is inert in the
  main checkout and bootstraps linked worktrees once.

For each linked worktree, the bootstrap:

- copies warm local caches from the main checkout when present:
  `.dependencies/` and `.direnv/`;
- creates a local `.envrc` with `use flake` if the worktree does not already
  have one;
- writes `.env.worktree` with stable, slot-based `PORT`, `PG_PORT`,
  `COMPOSE_PROJECT_NAME`, and `AI_PLUGINS_MAIN_CHECKOUT` values.

This repo uses `just` as its local command wrapper. The underlying scripts are
plain shell so the worktrees plugin can adapt to repositories that use Make,
package-manager scripts, another runner, or no wrapper at all.

There are no long-running services or containers in this repo today, so
`scripts/worktree-teardown.sh` only loads `.env.worktree` and performs a Docker
Compose shutdown if a future workflow adds `COMPOSE_PROJECT_NAME`-scoped
services. Remove worktrees through:

```shell
just worktree-teardown .worktrees/<branch-name>
```

Port allocation is stable per worktree and recorded under Git's common
directory. Override defaults with `WORKTREE_PORT_BASE_HTTP`,
`WORKTREE_PORT_BASE_PG`, and `WORKTREE_PORT_STRIDE` before bootstrap if needed.

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
   `plugins` array. `source` is the **explicit relative path** to the plugin
   directory, `./plugins/<plugin-name>` (do not use a bare directory name with
   `metadata.pluginRoot` — some Claude Code versions reject that as an
   unsupported source type and treat the plugin as remote). Mirror the entry in
   `.agents/plugins/marketplace.json` for Codex (which uses the
   `{ "source": "local", "path": "./plugins/<plugin-name>" }` object form).
   ```json
   {
     "name": "<plugin-name>",
     "source": "./plugins/<plugin-name>",
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

## Engineering standards (harness-agnostic)

This project follows a strict, documented engineering regime. The canonical rules
live in [`docs/rules/`](docs/rules/) and every architectural decision is recorded
in [`docs/adr/`](docs/adr/). In brief: functional-core/imperative-shell design,
parse-don't-validate semantic types where the stack supports them,
railway-oriented errors, strict linting, behavior-focused tests, eval-driven
effectiveness and minimum-necessary context for skills/MCP, PR-based CI with
required approval, Conventional Commits with **no `Co-Authored-By` trailers**,
and no quality shortcuts. These rules apply to **both Claude Code and Codex**;
`CLAUDE.md` is a thin pointer to this file.

## CI/CD and release

CI runs on GitHub Actions (`.github/workflows/ci.yml`):

- **`ci.yml`** (PR + merge queue): `just ci`, marketplace
  validation (including the cross-harness manifest sync-validator), Codex
  manifest checks, and a final `CI gate` aggregator job so branch protection has
  a single required check.

## Reference

- Marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Plugin reference (full `plugin.json` schema): https://code.claude.com/docs/en/plugins-reference
- Creating plugins: https://code.claude.com/docs/en/plugins
- Discover & install: https://code.claude.com/docs/en/discover-plugins
