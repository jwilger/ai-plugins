# AGENTS.md

Guidance for AI agents (Codex, Claude Code, etc.) working in this repository.

## What this repo is

`ai-plugins` is a **multi-harness AI plugin marketplace**. It implements the
[Claude Code marketplace](https://code.claude.com/docs/en/plugin-marketplaces)
format and carries Codex-facing marketplace metadata and plugin manifests for
Codex and other harnesses that adopt the plugin concept.

Codex is the primary supported harness; Claude Code is supported from the same
plugin tree.

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
nix develop                       # provides node, npm, jq, prettier, rg, fd, just, bats, lefthook, bd
```

**Critical convention:** anything npm would normally install "globally" must
land in the git-ignored `./.dependencies/` directory, not in `$HOME`. The
devshell enforces this by setting `NPM_CONFIG_PREFIX` and `NPM_CONFIG_CACHE` to
point inside `./.dependencies/` and prepending the local npm `bin/` dir to
`PATH`. So:

- `npm install -g <pkg>` → installs to `./.dependencies/npm/`

Never commit `./.dependencies/`. If the environment looks broken, `rm -rf
.dependencies` and re-enter the devshell.

Promptfoo and the optional Claude and Codex evaluation SDKs are pinned in
`tooling/evals/package.json` and `tooling/evals/package-lock.json`.
`scripts/evals/ensure-node-deps.sh` installs them there and maintains the
git-ignored root `node_modules` symlink required by the evaluation tooling;
`scripts/evals/run.sh` and `scripts/evals/share.sh` restore it when missing.

`.envrc` (`use flake`) is git-ignored here per the maintainer's global config;
recreate it locally if you use direnv.

## Worktree workflow

Linked worktrees isolate **concurrent mutable tickets**, not ordinary work.
Questions and read-only investigation require neither a ticket nor a worktree.
One mutable ticket may use its current checkout. Create a linked worktree only
when separate mutable tickets are active concurrently across sessions, agents,
or subagents; never create a nested worktree merely for another task.

The active harness owns worktree creation, switching, and cleanup. Use its
normal linked-worktree workflow, then let this repository's bootstrap and
teardown helpers supply repository-specific environment setup and cleanup. A
worktree is optional isolation, not a delivery precondition.

Install or refresh the committed post-checkout bootstrap hook from the primary
checkout:

```shell
just worktree-hooks
```

The managed Lefthook configuration owns `post-checkout` and runs
`scripts/worktree-bootstrap.sh` for linked worktrees. It leaves ordinary
checkouts inert. The installer preserves foreign hooks and only removes its
own recognized legacy launchers. Rerun it after a behavior-affecting change to
`lefthook.yml`, `scripts/install-worktree-hooks.sh`, `flake.nix`, or
`flake.lock`.

`scripts/worktree-guard.sh` remains only as a no-op compatibility shim for
already-installed legacy pre-commit and pre-push launchers. It is not a
checkout-location policy and must not be used to decide whether work may
proceed.

For each linked worktree, the bootstrap:

- copies warm local dependency and devshell caches from the main checkout when
  present: `.dependencies/` and `.direnv/`; legacy `.dependencies/evals/`
  state is excluded, while current disposable eval state lives per worktree
  under the git-ignored `.evals/` directory;
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
services. When optional cleanup is appropriate, prepare it through:

```shell
just worktree-teardown .worktrees/<branch-name>
```

Then let the owner or active harness remove a clean worktree while preserving
its branch. Port allocation is stable per worktree and recorded under Git's
common directory. Override defaults with `WORKTREE_PORT_BASE_HTTP`,
`WORKTREE_PORT_BASE_PG`, and `WORKTREE_PORT_STRIDE` before bootstrap if needed.

## Backlog capacity management

Use Beads with its Dolt backend as the repository task board for planned mutable
delivery work, and manage queued work as a deliberately bounded backlog. Run
`bd prime` for current CLI guidance and use `--json` for programmatic reads.
Questions, read-only investigation, and ordinary explanation do not require a
ticket, a claim, or a worktree.

- The active issue (`in_progress`) does not count toward backlog capacity. A
  queued issue is an unclaimed `open` issue that is not blocked or deferred.
- Keep at most five queued tickets. Do not maintain an overflow, icebox, shadow
  backlog, or other hidden queue.
- Discovery identifies a candidate; it does not create an obligation to admit
  or retain a ticket.
- Compare candidates by user pain and frequency, severity, blocking impact,
  future leverage, confidence, value relative to cost, and overlap with existing
  root causes.
- Select ready issues deterministically by priority ascending, creation time
  ascending, then issue ID ascending. Re-rank the complete queue whenever an
  issue is admitted, combined, displaced, completed, reopened, or materially
  re-scoped; dependency edges represent real blocking relationships, not
  artificial ordering.
- When fewer than five tickets are queued, admit a worthwhile candidate
  normally. At capacity, evaluate a candidate before creating a ticket and
  choose exactly one explicit outcome: replace a lower-value queued ticket;
  combine genuinely overlapping tickets; or reject the candidate without
  creating a ticket. Record a concise reason for every combination,
  displacement, or rejection.
- When the backlog falls to two or fewer queued tickets, perform a replenishment
  review. Inspect durable memories, recent usage friction, eval failures, and
  recurring workarounds for worthwhile candidates. It is valid to add nothing.
- Blocking defects and in-model security issues required to complete the active
  ticket remain causal work within that ticket. Do not create separate backlog
  tickets merely to evade the cap.
- Work on one issue at a time. Before starting ready work, claim it atomically
  with `bd update <id> --claim`; after completing it, choose the first issue in
  the deterministic ready order. Use the delivery formula configured in
  `[beads].workflow` and attach behavior, documentation, CI-workflow, or
  validation-only slice molecules according to the changed surface.

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

Run provider-backed evals only for behavior that changed files could plausibly
affect. Case, condition, and harness selection are causal engineering choices,
not a ritual full-suite gate. `just evals` compares the branch with
`origin/main` and applies the repository mapping: shared skill prose selects
only cases targeting those skills across Codex and Claude Code; plugin, hook,
or harness behavior selects the relevant installed-plugin canary or outcome
scenario; documentation, tests, and unrelated implementation details select no
live eval. Every selected case defaults to one sample unless the named metric
requires repetition.

```shell
just evals
EVAL_BASE_REF=<ref> scripts/evals/run-changed.sh
EVAL_CASE_FILTER='<case-regex>' EVAL_PROVIDER_FILTER='<provider>' EVAL_SAMPLES=1 \
  scripts/evals/run.sh
nix develop -c node scripts/evals/build-site.mjs
```

Use `just evals-all` only as an explicit research experiment when a concrete
cross-case, cross-condition, cross-harness hypothesis requires the exhaustive
matrix. It is never a routine completion or release gate. `just evals` shares
fresh Promptfoo artifacts when its selected scope produces them; executable
outcome scenarios remain local evidence. If Promptfoo writes artifacts and
exits with failed evals, the command shares before returning the eval failure.
Interrupted or timed-out runs are not shared.

`scripts/evals/run.sh --dry-run` only validates promptfoo wiring and is useful
for pull-request CI without secrets; it is not behavior evidence. Provider-backed
runs require working authentication for only the selected harnesses. The runner restores
the pinned npm dev dependencies from `tooling/evals/package-lock.json`, generates promptfoo
config from the current marketplace manifests, prepares isolated no-plugin and
`development-system` homes, installs the Claude plugin through the real
marketplace CLI, and configures Claude with `apiKeyRequired: false`. Local
Claude subscription runs read the current access token into the eval process
while leaving the rotating refresh token in the source Claude config; both
plugin conditions therefore use disposable runtime config without copying
credentials or invalidating the source login. API-key or explicit-token runs
use the same isolated runtime config. The
runner uses Codex as the default model-graded assertion provider and disables
prompt response caching and hosted sharing so generated artifacts are fresh and repo-owned. Run
`scripts/evals/run.sh --suite canary` to prove installed `development-system`
loading and SessionStart execution before relying on behavior results. The optional Promptfoo MCP
server in the `agentic-systems-engineering` Codex manifest is for
agent-assisted validation, focused runs, and result inspection; it does not
replace the canonical runner.

`just evals` runs its change-selected scope in one Promptfoo process with up to
eight target calls globally. The existing Claude Code and Codex condition-specific
homes remain isolated within that process, and it writes the normal one-report
artifact set under `evals/out/` before sharing it. The cap is global, so it does
not guarantee a four-per-provider split.

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
provider-backed evals and benchmarks through the supported coding harnesses:

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

- Codex reads `.agents/plugins/marketplace.json` and per-plugin
  `.codex-plugin/plugin.json`. Treat the Codex manifest as the canonical version
  source for a plugin supported in both harnesses.
- Claude Code reads `.claude-plugin/marketplace.json` and per-plugin
  `.claude-plugin/plugin.json`. Keep shared Claude and Codex metadata versions
  synchronized.
- Codex-only plugins are allowed when Claude Code already provides equivalent
  built-in behavior; keep them out of Claude marketplace metadata and Claude
  behavior evaluation coverage. Prefer additive, harness-namespaced metadata
  rather than overloading either marketplace format.

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

## CI/CD and package lifecycle

Development-system is distributed through the Codex and Claude Code marketplace
manifests, not as an npm package. Do not add an npm publication workflow,
registry credential, trusted-publisher binding, publication-only version commit,
or package tag.

`plugins/development-system/.codex-plugin/plugin.json` is the canonical version
source. For a required version bump, update it and run
`node scripts/sync-development-system-metadata.mjs --write` to synchronize the
Claude manifest, marketplace entries, cache launchers, and catalog. Validate
the resulting plugin manifests and installed-harness behavior through the
causal checks selected by `just ci`.

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
