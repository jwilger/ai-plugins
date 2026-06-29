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

## Rust control plane (`crates/`)

The **sidequest** control plane lives in a Cargo workspace under `crates/`:

- `crates/sidequest-core` — the pure functional core (no I/O dependencies, by
  construction).
- `crates/sidequest` — the imperative shell, with two binaries: `sidequest-mcp`
  (the MCP stdio server, the primary surface) and `sidequest` (the CLI).

Work inside the Nix devshell and use `just` as the command interface:

```shell
nix develop      # nightly toolchain (rust-toolchain.toml) + just + cargo-nextest/mutants/audit + release-plz
just ci          # fmt-check + clippy -D warnings + nextest  (run before every commit)
just mutants     # mutation testing — 100% mutant kill required
```

Rust dependencies are managed **only via `cargo add`** (never hand-edit
`[dependencies]`).

## Engineering standards (harness-agnostic)

This project follows a strict, documented engineering regime. The canonical rules
live in [`docs/rules/`](docs/rules/) and every architectural decision is recorded
in [`docs/adr/`](docs/adr/). In brief: functional-core/imperative-shell with a
Step/Trampoline effect pattern; parse-don't-validate semantic types (`nutype`);
railway-oriented errors (`thiserror`); strict clippy; BDD/Cucumber one step at a
time in vertical slices; 100% mutation kill; eval-driven effectiveness and
minimum-necessary context for skills/MCP; PR-based CI with required approval and
managed release; Conventional Commits with **no `Co-Authored-By` trailers**; never
take quality shortcuts. These rules apply to **both Claude Code and Codex**;
`CLAUDE.md` is a thin pointer to this file.

## CI/CD and release

CI and release run on Forgejo Actions (`.forgejo/workflows/`):

- **`ci.yml`** (PR + push to `main`): the full `just ci` gate (build, fmt, clippy
  `-D warnings`, nextest, doctests, BDD, bats), marketplace validation
  (including the cross-harness manifest sync-validator), a `cargo-audit`
  security job, Codex-manifest checks, mutation testing on release PRs (keyed on
  the `release-plz-*` branch), and a final `gate` aggregator job so branch
  protection has a single required check.
- **Release is two-phase**, mirroring eventcore:
  - **`release-plz.yml`** (Phase 1, push to `main` except `chore(release):`
    commits): opens/updates a **signed** release PR. `main` rejects unverified
    commits, so `release-plz update` makes the file changes and the helper
    scripts in `.forgejo/scripts/` create the signed commit and open the PR via
    the forge API.
  - **`publish.yml`** (Phase 2, when the `chore(release):` merge lands): runs
    `release-plz release` to publish to crates.io and cut the Forgejo release.

Organization secrets/vars (available to the repo): `RELEASE_PLZ_TOKEN` (forge
PAT), `RELEASE_SIGNING_KEY` (SSH or GPG key), `CARGO_REGISTRY_TOKEN` (crates.io),
and `RELEASE_SIGNING_NAME` / `RELEASE_SIGNING_EMAIL` (vars). Publication order
(`release-plz.toml`): `sidequest-core` before `sidequest`. Branch protection
(≥1 approval, with auto_review contributing the approval) is a Forgejo
server-side setting.

## Reference

- Marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Plugin reference (full `plugin.json` schema): https://code.claude.com/docs/en/plugins-reference
- Creating plugins: https://code.claude.com/docs/en/plugins
- Discover & install: https://code.claude.com/docs/en/discover-plugins
