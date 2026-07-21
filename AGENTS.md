# AGENTS.md

Guidance for AI agents (Claude Code, Codex, etc.) working in this repository.

## What this repo is

`ai-plugins` is a **multi-harness AI plugin marketplace**. It implements the
[Claude Code marketplace](https://code.claude.com/docs/en/plugin-marketplaces)
format and carries Codex-facing marketplace metadata and plugin manifests for
Codex and other harnesses that adopt the plugin concept.

When this repository's marketplace plugins are installed in an agent harness,
use the relevant installed skills for matching work rather than treating plugin
content as inert documentation. In particular, route LLM, RAG, agent, tool-use,
structured-output, stochastic-eval, and agentic-delivery work through
`agentic-systems-engineering`; use `eval-case-reporter` when surprising or
borderline assistant behavior should become a scrubbed eval-case issue; and use
`engineering-standards` for the broader engineering regime. Eval-case reporting
must scrub/anonymize sensitive details, show the sanitized issue preview, and
require explicit user approval before posting. Never post raw secrets, private
client data, proprietary excerpts, auth material, or private transcripts.

- The Claude Code marketplace manifest is [`.claude-plugin/marketplace.json`](.claude-plugin/marketplace.json).
- The Codex marketplace manifest is [`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json).
- Each plugin is a subdirectory of [`plugins/`](plugins/).
- The user-facing catalog lives in [`README.md`](README.md), grouped by harness.

## Development environment

Use the Nix devshell — do not install global toolchains by hand.

```shell
nix develop                       # provides node, npm, jq, prettier, rg, fd, just, bats, lefthook
```

**Critical convention:** anything npm would normally install "globally" must
land in the git-ignored `./.dependencies/` directory, not in `$HOME`. The
devshell enforces this by setting `NPM_CONFIG_PREFIX` and `NPM_CONFIG_CACHE` to
point inside `./.dependencies/` and prepending the local npm `bin/` dir to
`PATH`. So:

- `npm install -g <pkg>` → installs to `./.dependencies/npm/`

Never commit `./.dependencies/`. If the environment looks broken, `rm -rf
.dependencies` and re-enter the devshell.

The Promptfoo eval runner is the exception to the "no root npm project" shape:
`package.json` and `package-lock.json` are committed so Promptfoo can resolve
its optional coding-harness provider SDKs from the project root. `node_modules/`
is git-ignored and restored with `npm ci`; `scripts/evals/run.sh` and
`scripts/evals/share.sh` run that restore automatically when Promptfoo, the
Codex SDK, or the Claude Agent SDK is missing.

`.envrc` (`use flake`) is git-ignored here per the maintainer's global config;
recreate it locally if you use direnv.

## Worktree workflow

This repo is configured for parallel development from linked worktrees. The main
checkout is the coordination checkout; feature work should happen in worktrees
created under the ignored repo-local `.worktrees/` directory:

```shell
git worktree add .worktrees/<branch-name> -b <branch-name>
```

Before making edits, agents should run:

```shell
scripts/agent-checkout-guard.sh
```

The guard exits successfully only from a linked worktree. In the main checkout
it blocks feature work, points to the linked-worktree command above, and
distinguishes ordinary local changes from the common case where the dirty
worktree already matches the upstream branch after a fetch.

Install the committed Lefthook configuration from the main checkout:

```shell
just worktree-hooks
```

Existing clones that installed the former direct shell hooks must rerun this
command after updating to the Lefthook migration. Rerun it after any
behavior-affecting change to `lefthook.yml` or
`scripts/install-worktree-hooks.sh`, and whenever `flake.nix` or `flake.lock`
changes the exported Lefthook runtime—even when its displayed version is
unchanged. Normal installation is deliberately refused from a linked worktree
because the installed runtime and configuration are shared by every worktree.

The Lefthook-managed hooks do two things:

- `pre-commit` and `pre-push` run `scripts/worktree-guard.sh`, which blocks
  commits and pushes from the main checkout while allowing linked worktrees.
- `post-checkout` runs `scripts/worktree-bootstrap.sh`, which is inert in the
  main checkout and bootstraps linked worktrees once.

The installer serializes concurrent runs with `flock`, registers the
flake-selected Lefthook store path as a repository-local Nix garbage-collection
root, validates and snapshots `lefthook.yml`, and replaces each launcher with an
atomic rename. Before replacing a foreign regular-file or symlink hook, it
copies that hook to the next unique `*.worktrees-backup` path. It does not
execute or chain those archival backups: inspect each reported backup and
migrate behavior that must remain active into `lefthook.yml` before deleting it.
If installation stops partway through, every hook path is still either the
complete old hook or the complete new launcher; fix the reported failure and
rerun `just worktree-hooks` to converge. `flock` releases automatically after
normal exit or a crash, and the next run removes abandoned staging directories.

`LEFTHOOK_CONFIG` pins the main snapshot, but Lefthook still merges a
checkout-local `lefthook-local.yml` into delegated jobs. Treat that file as an
intentional local override, not as part of the installed snapshot. The mandatory
worktree safety pass runs before Lefthook. Every launcher also passes
`--no-auto-install`, so an ordinary local `no_auto_install: false` override
cannot replace the repository-owned launcher.

Launchers derive Git's common directory at runtime and contain no checkout-path
literals. If a clone is moved, rerun `just worktree-hooks` from its new location
to repair the indirect Nix GC-root registration before the old auto-root is
garbage-collected.

The mandatory safety scripts themselves remain checkout-relative: a hook invokes
`scripts/worktree-guard.sh` or `scripts/worktree-bootstrap.sh` from the worktree
where Git ran it. A revision that predates or removes those scripts is not
hook-compatible and fails closed; do not treat runtime pinning as a promise that
arbitrary historical revisions can commit or push without the safety scripts.

Each launcher runs its mandatory worktree safety check once before delegating
to Lefthook, and the matching Lefthook job suppresses only that duplicate pass.
This keeps normal main-checkout enforcement and linked-worktree bootstrap
independent of Lefthook job selection while avoiding duplicate work.

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
2. Add a per-harness manifest for every marketplace that will list the plugin:
   `plugins/<plugin-name>/.claude-plugin/plugin.json` for Claude Code and
   `plugins/<plugin-name>/.codex-plugin/plugin.json` for Codex. Codex-only
   plugins must not carry a `.claude-plugin/plugin.json` or appear in the
   Claude Code marketplace. Only `name` is strictly required by some harnesses;
   prefer also setting `description`, `version` (semver), `author`, and
   `license`.
3. Put components at the **plugin root** (NOT inside `.claude-plugin/`):
   - `skills/<name>/SKILL.md` — adds to defaults; the primary mechanism for new work.
   - `agents/<name>.md` — subagents.
   - `commands/<name>.md` — legacy flat-file slash commands (prefer `skills/`).
   - `hooks/hooks.json`, `.mcp.json`, `.lsp.json`, `bin/` — as needed.
4. Register the plugin in the matching marketplace manifest(s). For Claude
   Code, append to `.claude-plugin/marketplace.json`; `source` is the
   **explicit relative path** to the plugin directory,
   `./plugins/<plugin-name>` (do not use a bare directory name with
   `metadata.pluginRoot` — some Claude Code versions reject that as an
   unsupported source type and treat the plugin as remote). For Codex, append to
   `.agents/plugins/marketplace.json` using the
   `{ "source": "local", "path": "./plugins/<plugin-name>" }` object form.
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
5. Add a row to each matching harness table in `README.md`; for Codex-only
   plugins, add only the Codex row.
6. Give the plugin its own `README.md` stating what it does and which
   harness(es) it targets.

## Validation (do this before claiming completion)

```shell
jq empty .claude-plugin/marketplace.json          # manifest is valid JSON
jq empty .agents/plugins/marketplace.json         # Codex manifest is valid JSON
find plugins -name plugin.json -exec jq empty {} \;  # every plugin manifest valid
prettier --check "**/*.{json,md}"                 # formatting (use --write to fix)
```

For every plugin in this marketplace, when modifying anything under `plugins/`
that could affect plugin or skill behavior, run the full relevant eval set
before claiming completion. Behavior evals for the marketplace run through
promptfoo's native Claude Code and Codex coding-agent providers, loading the
relevant marketplace surface for each harness:

```shell
just evals
nix develop -c scripts/evals/run.sh
nix develop -c node scripts/evals/build-site.mjs
```

`just evals` is the convenience path for local provider-backed evals plus
`promptfoo share`; it uploads the latest result and prints the share URL. Use
the lower-level commands when you need local-only artifacts or `promptfoo view`.
If Promptfoo writes artifacts and exits with failed evals, `just evals` still
shares and then returns the eval failure status. If the run is interrupted
with Ctrl-C, `just evals` exits immediately and does not share.

`scripts/evals/run.sh --dry-run` only validates promptfoo wiring and is useful
for pull-request CI without secrets; it is not behavior evidence. Provider-backed
runs require working Claude Code and Codex authentication. The runner restores
the pinned npm dev dependencies from `package-lock.json`, generates promptfoo
config from the current marketplace manifests, prepares a `CODEX_EVAL_HOME`
with every Codex marketplace plugin, configures Claude with `apiKeyRequired: false`, uses
Codex as the default model-graded assertion provider, and disables prompt
response caching and hosted sharing so generated artifacts are fresh and
repo-owned. Run `scripts/evals/run.sh --suite canary` to prove full-marketplace
plugin loading before relying on behavior results. The optional Promptfoo MCP
server in the `agentic-systems-engineering` Codex manifest is for
agent-assisted validation, focused runs, and result inspection; it does not
replace the canonical runner.

The static dashboard summarizes latest-run status by provider, case, sample,
plugin, and skill so PR notes can point to both aggregate quality and the
marketplace surface exercised by a scenario.

For Codex skills, "full" also includes analysis plus benchmark setup, and
benchmark execution when real scenarios and verifiers are available:

```shell
plugin-eval analyze plugins/<plugin-name>/skills/<skill-name> --format markdown
plugin-eval init-benchmark plugins/<plugin-name>/skills/<skill-name>
# After tailoring .plugin-eval/benchmark.json to real tasks:
plugin-eval benchmark plugins/<plugin-name>/skills/<skill-name> --config <benchmark.json>
```

If `plugin-eval` is not on `PATH`, run the installed plugin-eval script directly
from the local Codex plugin cache. If Claude Code has an equivalent evaluator for
the changed plugin or skill, run that too. Include eval results in the PR notes
alongside `just ci`. Do not wire provider-backed evals into untrusted PR gates
unless that automation is explicitly requested and secrets are protected.

### Standing authorization for repository-owned live evals

The repository owner grants standing approval to run repository-owned
provider-backed evals and benchmarks through both supported coding harnesses:

- Claude Code using the owner's existing Claude/Anthropic subscription authentication.
- Codex CLI using the owner's existing ChatGPT/OpenAI subscription authentication.

Local execution reuses those authenticated harness sessions and does not
require provider API keys or fresh approval merely because an authorized
repository-owned eval uses either provider. This authorization includes sending
this repository's purpose-built fixtures and prompts to the corresponding
provider. It does not authorize sending secrets, private client data,
proprietary unrelated content, or unrelated workspace files. Keep generated
authentication state isolated and disposable where the runner supports it,
including the generated Codex homes; leave the source harness logins untouched;
and run the repository's required secret-leak checks around every live
execution. Unattended trusted automation may use protected provider credentials
when it cannot reuse an interactive harness session. Never expose those
credentials to untrusted pull-request code or events.

This standing authorization covers the canonical downstream code-quality
benchmark command:

```shell
nix develop -c scripts/evals/run-code-quality-benchmark.sh
```

It does not authorize broad `nix develop` execution rules or other external
destinations.

This applies across all marketplace plugins, not only the plugin currently being
edited. Do not blanket-ignore `.plugin-eval/`. Stable benchmark configs and
curated eval baselines are useful review artifacts and may be committed when
they document how a plugin or skill is measured. Treat timestamped raw run logs
as transient unless you are intentionally adding a baseline for future
comparison.

When choosing sample counts, name the metric being measured. Prefer more
distinct cases for population quality. Use repeated samples deliberately for
per-input reliability, pass@k capability, pass^k reliability, stochastic judge
variance, or close A/B comparisons. Do not treat `k` as a ritual substitute for
better fixtures.

For a Claude-supported plugin, an end-to-end check in Claude Code is
`/plugin marketplace add .` then `/plugin install <plugin-name>@ai-plugins`.
Skip this for Codex-only plugins.

## Conventions

- **Names** are kebab-case, no spaces (marketplace `name`, plugin `name`,
  skill/agent directory and file names).
- **JSON** is 2-space indented; run `prettier --write` on changed `.json`/`.md`.
- **Only `.claude-plugin/`** lives inside the `.claude-plugin/` folder. All
  component directories (`skills/`, `agents/`, …) live at the plugin root.
- **Versioning:** every per-harness plugin manifest that exists must carry a
  valid semver `version`. For plugins listed in both harnesses, keep
  `.claude-plugin/plugin.json` and `.codex-plugin/plugin.json` versions
  identical, and keep the Claude Code marketplace entry version in
  `.claude-plugin/marketplace.json` identical to the Claude plugin manifest
  version. Codex-only plugins carry only `.codex-plugin/plugin.json`. Bump the
  plugin version in the same PR as any plugin behavior, skill, command, hook,
  script, or metadata change. Use semver: patch for fixes/documentation-only
  behavior clarifications, minor for backwards-compatible features or changed
  defaults, and major for breaking changes.

## Multi-harness notes

- Claude Code reads `.claude-plugin/marketplace.json` and per-plugin
  `.claude-plugin/plugin.json`. Keep these the source of truth.
- Codex reads `.agents/plugins/marketplace.json` and per-plugin
  `.codex-plugin/plugin.json`. Codex-only plugins are allowed when a harness
  already provides equivalent built-in behavior; keep them out of Claude Code
  marketplace metadata and Claude behavior eval coverage.
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
effectiveness and minimum-necessary context for skills/MCP, proportional threat
models derived from actual intended use, trunk push CI plus PR/merge-queue CI
with required approval in PR mode, Conventional Commits with **no
`Co-Authored-By` trailers**, and no quality shortcuts. If an hour passes without
a pushed commit, pause and challenge whether the current increment is
over-engineered; this is a scope heuristic, not permission to skip a gate. These
rules apply to **both Claude Code and Codex**;
`CLAUDE.md` is a thin pointer to this file.

## CI/CD and release

CI runs on GitHub Actions (`.github/workflows/ci.yml`):

- **`ci.yml`** (pushes to `main` + PR + merge queue): `just ci`, marketplace
  validation (including the cross-harness manifest sync-validator), Codex
  manifest checks, promptfoo eval dry-run wiring, and a final `CI gate`
  aggregator job so branch protection has a single required check.

## Reference

- Marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Plugin reference (full `plugin.json` schema): https://code.claude.com/docs/en/plugins-reference
- Creating plugins: https://code.claude.com/docs/en/plugins
- Discover & install: https://code.claude.com/docs/en/discover-plugins
