# ai-plugins

A multi-harness marketplace of AI coding-assistant plugins, with **Codex as the
primary harness** and **Claude Code supported** from the same plugin tree.

## Development system

This marketplace currently ships one installable plugin:
[`development-system`](plugins/development-system/README.md). It provides a
configured development workflow, engineering standards, review support, Beads
task coordination, and optional agentic-system and eval-reporting capabilities.
Codex and Claude Code load the same public skills and project policy.

The default preset is direct-to-trunk delivery with Beads backed by its embedded
Dolt store. The plugin supplies delivery, behavior, documentation, CI-workflow,
validation-only, and CI-recovery formulas; `.development-system.toml` selects
the enabled capabilities and delivery mode.

### Workflow at a glance

- Questions and read-only investigation need neither a Beads ticket nor a
  worktree.
- For planned mutable delivery work, use Beads as the task and workflow
  authority. Claim work atomically and follow the configured delivery formula.
- One mutable ticket may use its existing checkout. Create a linked worktree
  only when concurrent mutable tickets need isolation across sessions, agents,
  or subagents.
- Goals remain native to the active harness. Development-system does not create
  a second cross-harness goal state or continuation driver.
- The active harness owns worktree creation, switching, and cleanup. Repository
  bootstrap and teardown scripts remain available around a linked worktree when
  it is used.

## Plugin catalog

| Plugin                                                     | Harnesses          | Description                                                | Version |
| ---------------------------------------------------------- | ------------------ | ---------------------------------------------------------- | ------- |
| [development-system](plugins/development-system/README.md) | Codex, Claude Code | One configurable development workflow with Beads and Dolt. | 2.0.0   |

## Install development-system

### Codex (primary)

Codex-facing marketplace metadata is in
[`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json), and the
plugin manifest is at
[`plugins/development-system/.codex-plugin/plugin.json`](plugins/development-system/.codex-plugin/plugin.json).
To install from the GitHub marketplace source, run:

```shell
codex plugin marketplace add jwilger/ai-plugins
codex plugin add development-system@ai-plugins
```

For a local checkout, replace the first source with its marketplace root, for
example `codex plugin marketplace add ./ai-plugins`. Start a new Codex thread
after installation so its hooks and skills load for the target repository. Then
open `/hooks`, inspect, and explicitly trust the current Development System
hook definition. Repeat that review after a hook-definition or plugin update.
Do not use `--dangerously-bypass-hook-trust` merely to skip the review.

### Claude Code

Add the marketplace, then install the plugin from inside Claude Code:

```text
/plugin marketplace add jwilger/ai-plugins
/plugin install development-system@ai-plugins
```

For a local checkout, use its path in the marketplace-add command. The
marketplace name remains `ai-plugins` in the install command.

After either installation, begin from the target repository's primary checkout
and use the setup skill or a dry-run setup preview before approving repository
mutations. The plugin guide describes the workflow in detail.

## Optional integrations are owner-operated

Before considering adjunct integrations, use the read-only handoff guide:

```shell
development-system integrations --harness <all|codex|claude>
```

If the harness does not expose the plugin executable on `PATH`, invoke the same
command as `<plugin-root>/bin/development-system integrations …`.

It reports the ownership boundaries and the official installation path; it does
not download software, run an installer, alter harness configuration, or read
or write credentials.

- **Beads:** lifecycle hooks are plugin-owned and only run for a project that
  explicitly enables Beads. Do not add duplicate user hooks. On Codex, trust
  the plugin's current hook definition through `/hooks` after installation or
  an update.
- **Context7:** install it only through its official marketplace commands for
  the selected harness. Where Context7 asks for a key, the owner supplies
  `CONTEXT7_API_KEY` through their private environment or official
  configuration; no key is embedded in this marketplace.
- **Hindsight for Claude Code:** the owner may install the official Hindsight
  marketplace plugin after reviewing it.
- **Hindsight for Codex:** use Hindsight's official interactive installer or
  manual configuration. Upstream publishes `curl -fsSL
https://hindsight.vectorize.io/get-codex | bash`, but do not execute that
  direct pipeline: fetch the installer to a temporary file, inspect it, and
  separately approve execution. The installer overwrites
  `~/.codex/hooks.json`, so first back up any existing file, then merge the
  generated Hindsight hooks with backed-up non-Beads shared hooks. Omit legacy
  `bd`/Beads lifecycle entries: Development System is their sole owner. Review
  and explicitly trust the resulting Hindsight user-hook definitions through
  `/hooks`. Do not use its `--uninstall` mode when shared hooks exist because
  it deletes the file; preserve the backup and manually remove only Hindsight's
  entries. Do not have an agent execute the installer. The owner chooses cloud
  or local storage and supplies credentials privately.

No integration keys, tokens, retention settings, or memory-bank configuration
are embedded in development-system. Before enabling Hindsight, the owner must
make explicit decisions about what may be retained, the memory-bank scope, and
the retention policy.

## Developing in this repository

A [Nix flake](flake.nix) provides a reproducible devshell with Node, npm, `jq`,
`prettier`, `ripgrep`, `fd`, `just`, `bats`, Lefthook, and `bd`.

```shell
nix develop
```

Any npm tooling installed globally inside the devshell goes to the ignored
`./.dependencies/` directory rather than the user's home directory. Promptfoo
and the optional Codex and Claude evaluation SDKs are pinned in
[`tooling/evals/package.json`](tooling/evals/package.json) and its lockfile.
The eval scripts restore their ignored dependency tree when needed.

See [`AGENTS.md`](AGENTS.md) for repository workflow, plugin authoring,
validation, and delivery guidance.

## Eval reports

The repository-owned evaluation dashboard is generated under `site/evals/` by
`node scripts/evals/build-site.mjs`. It records the latest provider, case,
sample, installed composition, and target plugin/skill results without relying
on hosted report storage.

Canonical behavior evaluation uses the Claude Agent SDK and Codex SDK with
isolated no-plugin and installed-development-system conditions. The runner
reuses an existing Claude Code or Codex subscription session when available;
untrusted pull-request checks remain secret-free and run only configuration and
dry-run validation.

Provider-backed evaluation is change-scoped by default:

```shell
just evals
EVAL_BASE_REF=<ref> scripts/evals/run-changed.sh
EVAL_CASE_FILTER='<case-regex>' EVAL_PROVIDER_FILTER='<provider>' EVAL_SAMPLES=1 \
  scripts/evals/run.sh
nix develop -c node scripts/evals/build-site.mjs
```

Shared skill changes select only their mapped cases across Codex and Claude
Code. Plugin, hook, or harness behavior selects the relevant installed-plugin
canary or outcome scenario; documentation and unrelated implementation changes
select no live evaluation. `just evals-all` is an explicit research experiment,
not a routine completion gate.

`just evals` runs its change-selected scope in one Promptfoo process with a
global cap of eight target calls. It preserves the existing Claude Code and
Codex condition-specific homes, writes the normal one-report artifact set under
`evals/out/`, and shares that report after a completed or non-interrupted run.
The cap is global rather than a guaranteed four-per-provider split, and semantic
rubric grading still uses Codex as additional provider traffic.

Use repeated samples only for a named reliability, pass@k, pass^k, or judge
variance metric. Prefer more distinct cases when measuring population quality.

## Reporting eval cases

When a plugin, skill, prompt, or workflow behaves incorrectly or only partly
works, file an **Eval case** issue for a future regression fixture. Include
sanitized input, actual and expected behavior, expected outcome, and the
assertion or rubric that would detect it. Never include credentials, session
material, private client data, private repository names, internal hostnames, or
raw proprietary excerpts.

## Repository layout

```text
.
├── .agents/plugins/          # Codex marketplace metadata
├── .claude-plugin/           # Claude Code marketplace metadata
├── docs/                     # ADRs, research, and plans
├── evals/                    # behavior fixtures and promptfoo support
├── plugins/                  # one directory per plugin
├── scripts/                  # validation, worktree, and eval helpers
├── tooling/evals/            # pinned eval dependencies
├── AGENTS.md                 # repository guidance
└── README.md                 # this file
```

## License

See the plugin manifest for its license.
