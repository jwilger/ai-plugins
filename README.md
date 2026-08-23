# ai-plugins

A **multi-harness marketplace of AI coding-assistant plugins** for
[Claude Code](https://code.claude.com), [Codex](https://openai.com/codex/), and
other harnesses that adopt plugin or marketplace concepts.

## Personal development system

This marketplace has one audience and one installable plugin:
[`development-system`](plugins/development-system/README.md). It supports Codex
and Claude Code with one initialization command and one project configuration
file.

The default preset is direct-to-trunk delivery with Tiber and on-demand linked
worktrees for concurrent mutable work.
Optional agentic-system and eval-reporting capabilities are selected in
`.development-system.toml`; the plugin owns its bundled MCP surface.

The strong recommendation is to install only `development-system`. Additional
plugin marketplaces expand the supply-chain trust surface. The SessionStart
hook warns about conflicting plugins, incompatible harness settings, and
user-managed MCPs that need compatibility review.

## Plugin catalog

| Plugin                                                     | Harnesses          | Description                                                                                          | Version |
| ---------------------------------------------------------- | ------------------ | ---------------------------------------------------------------------------------------------------- | ------- |
| [development-system](plugins/development-system/README.md) | Codex, Claude Code | Advisory repository setup and structured multi-agent review with reusable native services for Tiber. | 5.3.2   |

## Using the marketplace (Claude Code)

Add this repository as a marketplace, then install a plugin from it:

```shell
# From inside Claude Code:
/plugin marketplace add jwilger/ai-plugins      # GitHub owner/repo shorthand
# ...or a local checkout:
/plugin marketplace add ./ai-plugins

/plugin install development-system@ai-plugins
```

After every install or upgrade, build the two local Rust executables from the
marketplace checkout for the current host:

```shell
just install-development-system-binaries
```

The bootstrap installs the binaries under
`$XDG_DATA_HOME/ai-plugins/development-system/` (or
`~/.local/share/ai-plugins/development-system/`) by plugin version and host.
Normal plugin use never invokes Cargo.
The bootstrap itself requires a working Cargo environment; the repository Nix
devshell is optional convenience, not a plugin prerequisite. Without `just`,
run `scripts/install-development-system-binaries.sh` from the checkout.

The marketplace is referenced by its **name** (`ai-plugins`) in install
commands, regardless of the URL you added it from. List and manage with
`/plugin list`, `/plugin marketplace update ai-plugins`, and
`/plugin marketplace remove ai-plugins`.

## Using the marketplace (Codex)

Codex-facing marketplace metadata lives in
[`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json), and each
plugin has a `.codex-plugin/plugin.json` manifest. In a local checkout, install
or sync the plugin from the matching directory under [`plugins/`](plugins/)
using the Codex plugin flow available in your Codex environment.

Install `development-system` from the local marketplace, then start a new
thread and run its setup skill from the target repository's primary checkout.
The setup skill installs the required current-host binaries before configuring
the repository. You can also run `just install-development-system-binaries`
manually from the marketplace checkout.

## Developing in this repo

A [Nix flake](flake.nix) provides a reproducible devshell with Node, npm, `jq`,
`prettier`, `ripgrep`, `fd`, `just`, and `bats`.

```shell
nix develop        # enter the devshell
# or, with direnv:
echo "use flake" > .envrc && direnv allow
```

Any **globally installed** npm tooling (`npm install -g …`) is redirected into a
git-ignored `./.dependencies/` directory by the devshell, so it never pollutes
your home directory. Delete that directory any time for a clean slate.

This repo also has a committed `package.json`/`package-lock.json` for the local
Promptfoo eval runner. `node_modules/` is ignored and restored with `npm ci`;
the eval scripts run that automatically when the Promptfoo, Codex SDK, or
Claude Agent SDK packages are missing.

See [`AGENTS.md`](AGENTS.md) for how to author, validate, and publish a plugin.

## Eval reports

The repo-owned eval dashboard is generated under `site/evals/` by
`node scripts/evals/build-site.mjs`. It is a local/static artifact for review
and workflow uploads; the durable record is repo-owned and does not depend on
promptfoo-hosted sharing.

Local runs reuse existing Claude Code/Anthropic and Codex/ChatGPT subscription
sessions. They do not require provider API keys or fresh approval for the
repository-owned evals authorized in [`AGENTS.md`](AGENTS.md). Unattended trusted
automation may instead use protected provider credentials when interactive
harness sessions are unavailable; untrusted pull-request checks remain
secret-free and validate only the eval configuration and dry-run wiring.

The dashboard includes latest-run status, provider/case/sample pass rates,
threshold status, exact installed provider compositions, and separate
case-target plugin/skill summaries so regressions can be traced back to both the
loaded marketplace surface and the behavior each scenario exercises.

The canonical promptfoo behavior evals run through Promptfoo's native coding
agent providers: `anthropic:claude-agent-sdk` for Claude Code and
`openai:codex-sdk` for Codex. The runner generates the promptfoo config from
the current marketplace manifests and labels no-plugin, targeted-plugin, and
full-marketplace behavior modes. Codex uses a separate generated home for each
mode. For both harnesses, targeted mode installs the deterministic, deduplicated
union of plugins declared by the selected behavior cases; `EVAL_CASE_FILTER`
therefore narrows both the cases and their installed plugin set. Full-marketplace
mode still installs the complete harness-specific catalog, while no-plugin mode
installs none. The generated config records each provider's exact installed
composition separately from the plugins targeted by an individual case. An
unfiltered targeted run equals the full catalog in both harnesses today because
the marketplace has one public plugin and the selected cases target it. The two
modes remain distinct controls for filtered runs and future catalog changes.
Promptfoo is pinned at `0.121.18`;
the Promptfoo, Codex SDK, and Claude Agent SDK packages are pinned in
`package.json` and `package-lock.json`. The runner disables prompt response
caching and hosted sharing so a behavior run is a fresh local record.

Default eval harness posture:

- Claude Code: `anthropic:claude-agent-sdk`, Sonnet 5 via the `sonnet` alias,
  local Claude Code authentication via `apiKeyRequired: false`, and all local
  plugins with `skills: all`. The intended human-facing Claude Code posture
  remains Sonnet high effort with Opus 4.8 advisor where that harness exposes
  those controls; Promptfoo's current Claude Agent SDK provider does not expose
  those knobs in this repo's generated config.
- Codex execution: `openai:codex-sdk`, `gpt-5.6-terra` with
  `model_reasoning_effort=medium`, read-only sandbox, no approvals, streaming,
  deep tracing disabled, and isolated generated homes containing no plugins,
  the selected cases' deterministic plugin union, or the complete
  harness-specific catalog according to the behavior mode.
  Model-graded assertions independently default to
  `gpt-5.6-sol` with high reasoning through the same SDK, so OpenAI model access
  goes through local Codex auth rather than `OPENAI_API_KEY`. Override the two
  roles separately with `CODEX_EVAL_MODEL` / `CODEX_EVAL_REASONING_EFFORT` and
  `CODEX_GRADER_MODEL` / `CODEX_GRADER_REASONING_EFFORT`.

The focused [GPT-5.6 model-family benchmark](evals/benchmarks/gpt-5.6-model-family/README.md)
compares Sol, Terra, and Luna without running the full marketplace eval suite.
Its trace-enforced Codex app-server wrapper and skills-only/no-plugin homes are
benchmark controls; the canonical behavior runner above continues to use the
native Codex SDK provider and the configured behavior-mode matrix.

The canary suite is separate from behavior evals. Canaries may explicitly ask
the harness to prove plugin and skill loading. Behavior prompts stay natural and
do not tell the model to use this repository's plugins.

Repeated samples are a deliberate measurement choice, not a blanket rule. The
default one-sample matrix treats every case as a binary pass/fail observation
and estimates population quality across distinct cases. Set `EVAL_SAMPLES`
above one only when measuring per-input reliability, pass@k capability, pass^k
reliability, judge variance, or a small stochastic difference; fractional
per-case thresholds apply only to those repeated runs. PR dry-runs do not run
live samples.

Pull-request CI validates the eval configuration with `--dry-run` but does not
claim behavior evidence. Provider-backed behavior evidence comes from local,
scheduled, manual, or `main` runs where Claude Code and Codex authentication are
available.

To produce the same artifacts locally:

```shell
just evals  # runs provider-backed evals, shares the result, and prints the URL
nix develop -c scripts/evals/run.sh
nix develop -c scripts/evals/run.sh --suite canary
nix develop -c node scripts/evals/build-site.mjs
```

Eval runs have no implicit whole-run deadline: a large matrix must not lose
nearly-complete work to an arbitrary wall-clock cutoff. Set `EVAL_TIMEOUT`
only when the caller deliberately wants a bounded run. Timed-out or interrupted
runs write `evals/out/status.json` so the dashboard can show why no fresh
result completed.

`just evals` uploads the latest eval result through `promptfoo share`. For a
local-only report, run `scripts/evals/run.sh` and then
`nix develop -c node_modules/.bin/promptfoo view`. If a behavior eval exits
with Promptfoo's normal failure status after writing artifacts, `just evals`
still attempts to share the report and then returns the original eval status. If
the eval run is interrupted, terminated, or times out, `just evals` stops
without sharing. Interrupted, terminated, and timed-out runs all retain any
partial artifacts under
`evals/out/timeout-artifacts/` for debugging.

The repository retains an optional Promptfoo MCP definition under
`plugins/development-system/components/agentic-systems-engineering/`. That path
is an internal component identity, not a public marketplace plugin or skill
name; users install only `development-system`. A project that explicitly wires
the optional server must provide `promptfoo@0.121.18` on `PATH`. When the
project uses `flake.nix`, prefer `pkgs.promptfoo` when nixpkgs provides the
required version so updates flow through the flake lockfile; otherwise use the
project's local package-manager sandbox. The server supports agent-assisted
config validation, focused eval runs, result inspection, and fixture
development. It supplements the canonical runner; it does not replace the
repo-owned artifact path above. Promptfoo's separate `mcp` provider is for
testing MCP servers as systems under test and should be added only when a plugin
or project exposes an MCP server to evaluate.

If Codex or Claude Code reports a missing Development System binary, run
`just install-development-system-binaries` from the matching
marketplace checkout. For an explicitly configured Promptfoo server, also
verify that the pinned runtime above is available on `PATH`.

## Reporting eval cases

When a plugin, skill, prompt, or workflow behaves incorrectly or only partially
works, file an **Eval case** issue in this repository. Eval cases are the intake
path for future regression fixtures in `evals/fixtures/`.

Include the sanitized input, actual behavior, expected behavior, expected eval
outcome (`pass`, `fail`, `partial`, `adversarial`, or `unsure`), and the
assertion or rubric that would catch the behavior. Do not include secrets,
credentials, auth headers, cookies, session ids, private keys, private client
data, private repository names, internal hostnames, or raw proprietary source
excerpts.

## Repository layout

```text
.
├── .agents/
│   └── plugins/
│       └── marketplace.json  # Codex-facing marketplace manifest
├── .claude-plugin/
│   └── marketplace.json      # Claude Code marketplace manifest
├── .github/
│   ├── ISSUE_TEMPLATE/       # eval-case intake form
│   └── workflows/            # CI and eval workflows
├── docs/
│   └── superpowers/plans/    # implementation plans for larger changes
├── evals/
│   ├── fixtures/             # behavior eval scenarios
│   └── promptfoo/            # promptfoo loaders and assertions
├── plugins/                  # one subdirectory per plugin
├── scripts/
│   ├── evals/                # eval config generator, runner, and dashboard builder
│   └── tests/                # Bats tests
├── site/
│   └── evals/                # generated dashboard target, ignored except .gitkeep
├── flake.nix                 # Nix devshell
├── AGENTS.md                 # guidance for AI agents working in this repo
└── README.md                 # this file
```

## License

See individual plugins for their licenses.
