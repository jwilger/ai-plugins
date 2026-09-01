# AGENTS.md

Guidance for Codex agents working in this repository.

## What this repo is

`ai-plugins` is a **Codex plugin marketplace**. It carries Codex marketplace
metadata and plugin manifests.

When this repository's marketplace plugins are installed in an agent harness,
use the relevant installed skills for matching work rather than treating plugin
content as inert documentation. In particular, route LLM, RAG, agent, tool-use,
structured-output, stochastic-eval, and agentic-delivery work through
`development-system:agentic-systems`; use
`development-system:eval-case-reporting` when surprising or borderline
assistant behavior should become a scrubbed eval-case issue; and use
`development-system:engineering-standards` for the broader engineering regime.
Eval-case reporting must scrub/anonymize sensitive details, show the sanitized
issue preview, and require explicit user approval before posting. Never post
raw secrets, private client data, proprietary excerpts, auth material, or
private transcripts.

- The Codex marketplace manifest is [`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json).
- Each plugin is a subdirectory of [`plugins/`](plugins/).
- The user-facing catalog lives in [`README.md`](README.md).

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
its coding-agent provider SDK from the project root. `node_modules/`
is git-ignored and restored with `npm ci`; `scripts/evals/run.sh` and
`scripts/evals/share.sh` run that restore automatically when Promptfoo, the
Codex SDK is missing.

`.envrc` (`use flake`) is git-ignored here per the maintainer's global config;
recreate it locally if you use direnv.

## Worktree workflow

Linked worktrees are optional isolation for separate mutable tasks that overlap
across sessions or agents. Questions, read-only work, and one mutable task may
use the current checkout, including the primary checkout. Create a worktree
under the ignored repo-local `.worktrees/` directory when concurrency or an
explicit user request calls for one:

```shell
git worktree add .worktrees/<branch-name> -b <branch-name>
```

Before creating one, compare Git's absolute `--git-dir` and `--git-common-dir`;
reuse an existing linked checkout rather than nesting another. Verify
`.worktrees/` is ignored, and preserve unrelated existing changes by isolating
new work instead of moving or rewriting those changes.

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

- `pre-commit` runs the repository's fast local quality gate.
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
intentional local override, not as part of the installed snapshot. Every
launcher passes `--no-auto-install`, so an ordinary local
`no_auto_install: false` override cannot replace the repository-owned launcher.

Launchers derive Git's common directory at runtime and contain no checkout-path
literals. If a clone is moved, rerun `just worktree-hooks` from its new location
to repair the indirect Nix GC-root registration before the old auto-root is
garbage-collected.

The post-checkout launcher invokes `scripts/worktree-bootstrap.sh` from the
checkout where Git ran it before delegating to Lefthook. The retired
`scripts/worktree-guard.sh` remains a no-op compatibility shim until old managed
launchers have been refreshed with `just worktree-hooks`.

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
services. Remove worktrees through:

```shell
just worktree-teardown .worktrees/<branch-name>
```

Port allocation is stable per worktree and recorded under Git's common
directory. Override defaults with `WORKTREE_PORT_BASE_HTTP`,
`WORKTREE_PORT_BASE_PG`, and `WORKTREE_PORT_STRIDE` before bootstrap if needed.

## Backlog management

Use Tiber as the repository task board. This repository intentionally leaves
Tiber's optional backlog-capacity setting unset; there is no arbitrary numeric
limit on worthwhile queued work.

- A queued ticket is a ticket in `backlog` status. The active ticket is
  `in-progress`.
- Discovery identifies a candidate; it does not create an obligation to admit
  or retain a ticket.
- Compare candidates by user pain and frequency, severity, blocking impact,
  future leverage, confidence, value relative to cost, and overlap with existing
  root causes.
- Keep queued tickets in one strict priority order with no ties. Re-rank the
  complete queue whenever a ticket is admitted, combined, displaced, completed,
  reopened, or materially re-scoped; do not use creation order as priority.
- Admit worthwhile, materially distinct candidates and combine or reject
  overlapping work. Record a concise reason for every combination or rejection.
- Blocking defects and in-model security issues required to complete the active
  ticket remain causal work within that ticket. Do not create separate backlog
  tickets merely to fragment that causal work.
- Work on one ticket at a time. Before starting a queued ticket, move it to
  `in-progress`; after completing it, select the highest-priority queued ticket
  whose prerequisites are satisfied. If the highest-priority ticket is blocked,
  keep its priority explicit and start the highest-priority unblocked ticket.

## Adding a plugin

1. Create `plugins/<plugin-name>/` (kebab-case, no spaces — the name is
   public-facing and used for namespacing, e.g. `/<plugin-name>:<skill>`).
2. Add `plugins/<plugin-name>/.codex-plugin/plugin.json`. Prefer setting
   `name`, `description`, `version` (semver), `author`, and `license`.
3. Put components at the **plugin root**:
   - `skills/<name>/SKILL.md` — adds to defaults; the primary mechanism for new work.
   - `agents/<name>.toml` — Codex subagents.
   - `commands/<name>.md` — legacy flat-file slash commands (prefer `skills/`).
   - `hooks/codex.json`, `.lsp.json`, `bin/` — as needed.
4. Register the plugin in `.agents/plugins/marketplace.json` using the
   `{ "source": "local", "path": "./plugins/<plugin-name>" }` object form.
   ```json
   {
     "name": "<plugin-name>",
     "source": {
       "source": "local",
       "path": "./plugins/<plugin-name>"
     },
     "description": "…",
     "version": "0.1.0",
     "keywords": ["…"],
     "category": "…"
   }
   ```
5. Add a row to the plugin table in `README.md`.
6. Give the plugin its own `README.md` stating what it does.

## Validation (do this before claiming completion)

```shell
jq empty .agents/plugins/marketplace.json         # Codex manifest is valid JSON
find plugins -name plugin.json -exec jq empty {} \;  # every plugin manifest valid
prettier --check "**/*.{json,md}"                 # formatting (use --write to fix)
```

Run provider-backed behavior evals only when a change can plausibly alter
model-mediated behavior: skill/command/agent instructions, triggers or
descriptions, injected context, MCP tool schemas/descriptions/results consumed
by a model, prompt assembly, model/provider routing, hooks that change model
context, or the behavior fixtures and graders themselves. Do not run live LLM
evals merely because a changed file lives under `plugins/` or because a plugin
version changed.

Deterministic plugin infrastructure does not require provider-backed evals when
the changed behavior is fully exercised without a model. Examples include
installer paths and locking, launcher process/state handling, cache locations,
manifest/version synchronization, packaging, filesystem permissions, and
documentation that does not change agent instructions. For those changes, run
the relevant unit/integration/acceptance tests, manifest validation, and eval
configuration dry run. In delivery notes, state that live behavior evals were
not applicable and name the deterministic evidence instead.

When live evals are applicable, choose the smallest suite, cases, and sample
count that measure the changed claim. Use the full-marketplace set only when
the change affects a shared model-facing surface or when end-to-end marketplace
loading is itself the claim. Canonical live behavior evals run through
promptfoo's native Codex coding-agent provider.

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
runs require working Codex authentication. The runner restores the pinned npm
dev dependencies from `package-lock.json`, generates promptfoo config from the
current Codex marketplace manifest, prepares a `CODEX_EVAL_HOME` with every
Codex marketplace plugin, uses Codex as the default model-graded assertion
provider, and disables prompt response caching and hosted sharing so generated
artifacts are fresh and repo-owned. Run `scripts/evals/run.sh --suite canary`
to prove full-marketplace
plugin loading before relying on behavior results.

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
from the local Codex plugin cache.
Include applicable eval results in the PR notes alongside `just ci`. Do not wire
provider-backed evals into untrusted PR gates unless that automation is
explicitly requested and secrets are protected.

### Standing authorization for repository-owned live evals

The repository owner grants standing approval to run repository-owned
provider-backed evals and benchmarks through Codex CLI using the owner's
existing ChatGPT/OpenAI subscription authentication.

Local execution reuses that authenticated harness session and does not
require provider API keys or fresh approval merely because an authorized
repository-owned eval uses Codex. This authorization includes sending
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

## Conventions

- **Names** are kebab-case, no spaces (marketplace `name`, plugin `name`,
  skill/agent directory and file names).
- **JSON** is 2-space indented; run `prettier --write` on changed `.json`/`.md`.
- Component directories (`skills/`, `agents/`, …) live at the plugin root.
- **Versioning:** every `.codex-plugin/plugin.json` must carry a valid semver
  `version`. Keep each Codex marketplace entry version identical to its plugin
  manifest version. Bump the
  plugin version in the same PR as any plugin behavior, skill, command, hook,
  script, or metadata change. Use semver: patch for fixes/documentation-only
  behavior clarifications, minor for backwards-compatible features or changed
  defaults, and major for breaking changes.

## Codex marketplace notes

- Codex reads `.agents/plugins/marketplace.json` and per-plugin
  `.codex-plugin/plugin.json`; keep those surfaces synchronized.
- This repository does not support other agent harnesses. Do not add parallel
  manifests or compatibility behavior without a new architectural decision.

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
rules apply to Codex.

## CI/CD and release

CI runs on GitHub Actions (`.github/workflows/ci.yml`):

- **`ci.yml`** (pushes to `main` + PR + merge queue): `just ci`, marketplace
  validation, Codex manifest checks, promptfoo eval dry-run wiring, and a final `CI gate`
  aggregator job so branch protection has a single required check.
