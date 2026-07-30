# ai-plugins

A **multi-harness marketplace of AI coding-assistant plugins** for
[Pi](https://pi.dev), [Claude Code](https://code.claude.com), and
[Codex](https://openai.com/codex/) from one canonical package tree.

## Personal development system

This marketplace has one audience and one installable plugin:
[`development-system`](plugins/development-system/README.md). Pi is the primary
recommended surface, Claude Code is secondary, and Codex is tertiary. They use
one project configuration and the same eight physical skill files.

The default preset is direct-to-trunk delivery with linked worktrees and Tiber.
Optional agentic-system and eval-reporting capabilities are selected in
`.development-system.toml`; the plugin owns its bundled MCP surface.

The strong recommendation is to install only `development-system`. Additional
plugin marketplaces expand the supply-chain trust surface. Each SessionStart
hook warns only about conflicting plugins, incompatible settings, and
user-managed MCPs for the harness that is starting.

## Plugin catalog

| Plugin                                                     | Harnesses              | Description                                                         | Version |
| ---------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------- | ------- |
| [development-system](plugins/development-system/README.md) | Pi, Claude Code, Codex | One configurable development workflow with deterministic Pi guards. | 1.10.0  |

## Using the package (Pi — primary)

Pi packages execute trusted extension code with the user's full permissions.
Review an exact repository commit, then install that reviewed revision directly
from Git:

```shell
pi install git:github.com/jwilger/ai-plugins@<reviewed-commit>
```

To opt into following future default-branch revisions without reviewing each
one first, use `pi install git:github.com/jwilger/ai-plugins`. For development
from a local checkout, run the clean bootstrap and install the package
subdirectory:

```shell
nix develop -c scripts/bootstrap-pi-package.sh
pi install ./plugins/development-system
```

See the [development-system guide](plugins/development-system/README.md) for
local installation, project trust, setup, updates, status, target support,
capability differences, and removal.

## Validating the Pi package

This repository does not publish `@jwilger/development-system-pi` to npmjs.org.
Registry releases, publication tags, and automated release-version commits are
not part of the supported lifecycle. The checked-in package remains an
npm-format artifact because Pi uses its manifest and resource inventory for
local installation.

CI validates both the package payload and an exact pack, extract, and load
canary. Run the same provider-free checks locally:

```shell
nix develop -c just npm-package
nix develop -c just npm-package-canary
```

## Using the marketplace (Claude Code — secondary)

Add this repository as a marketplace, then install a plugin from it:

```shell
# From inside Claude Code:
/plugin marketplace add jwilger/ai-plugins      # GitHub owner/repo shorthand
# ...or a local checkout:
/plugin marketplace add ./ai-plugins

/plugin install development-system@ai-plugins
```

The marketplace is referenced by its **name** (`ai-plugins`) in install
commands, regardless of the URL you added it from. List and manage with
`/plugin list`, `/plugin marketplace update ai-plugins`, and
`/plugin marketplace remove ai-plugins`.

## Using the marketplace (Codex — tertiary)

Codex-facing marketplace metadata lives in
[`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json), and each
plugin has a `.codex-plugin/plugin.json` manifest. In a local checkout, install
or sync the plugin from the matching directory under [`plugins/`](plugins/)
using the Codex plugin flow available in your Codex environment.

Install `development-system` from the local marketplace, then start a new
thread and run its setup skill from the target repository's primary checkout.

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

The root `package.json` is the lightweight Pi Git-package facade. Promptfoo and
its optional coding-harness provider SDKs are pinned separately in
`tooling/evals/package.json` and `tooling/evals/package-lock.json`. Their
`node_modules/` tree is ignored and exposed through a generated root symlink so
existing module-resolution paths remain stable; the eval scripts restore it
automatically when dependencies are missing.

See [`AGENTS.md`](AGENTS.md) for how to author, validate, and publish a plugin.

## Eval reports

The repo-owned eval dashboard is generated under `site/evals/` by
`node scripts/evals/build-site.mjs`. It is a local/static artifact for review
and workflow uploads; the durable record is repo-owned and does not depend on
promptfoo-hosted sharing.

Local runs reuse existing Pi/OpenAI, Claude Code/Anthropic, and Codex/ChatGPT
subscription sessions. They do not require provider API keys or fresh approval for the
repository-owned evals authorized in [`AGENTS.md`](AGENTS.md). Unattended trusted
automation may instead use protected provider credentials when interactive
harness sessions are unavailable; untrusted pull-request checks remain
secret-free and validate only the eval configuration and dry-run wiring.

The dashboard includes latest-run status, provider/case/sample pass rates,
threshold status, exact installed provider compositions, and separate
case-target plugin/skill summaries so regressions can be traced back to both the
loaded marketplace surface and the behavior each scenario exercises.

The canonical Promptfoo behavior evals use a repository-owned Pi JSON provider,
`anthropic:claude-agent-sdk` for Claude Code, and `openai:codex-sdk` for Codex.
Pi has no-package, targeted development-system, and inventory-derived full
marketplace conditions. Claude Code and Codex retain no-plugin and installed
`development-system` conditions. Claude's
condition is prepared through the real marketplace installer and then loaded
from its installed cache path; Codex also uses the real marketplace installer
to populate an isolated generated home and plugin cache. Claude subscription
authentication reads the current access token into the eval process while
leaving the rotating refresh token in the owner's normal config. Neither
condition copies credentials, and both use disposable runtime config; API-key
or explicit-token runs use the same isolation. The live
runner also requires the plugin's SessionStart hook to execute.
The generated config records each provider's installed composition separately
from the plugins and skills targeted by an individual case.
Pi is pinned at `0.82.1` and Promptfoo is pinned at `0.121.19`;
the Promptfoo, Codex SDK, and Claude Agent SDK packages are pinned in
`tooling/evals/package.json` and `tooling/evals/package-lock.json`. The runner disables prompt response
caching and hosted sharing so a behavior run is a fresh local record.

Default eval harness posture:

- Pi: `openai-codex/gpt-5.6-terra` at medium reasoning through the owner's
  ChatGPT subscription. Each package condition has a disposable, single-writer
  Pi home; only the OpenAI auth entry is copied with mode 0600, extension source
  provenance is required, source auth integrity is checked, and copied stores
  are deleted after execution.
- Claude Code: `anthropic:claude-agent-sdk`, Sonnet 5 via the `sonnet` alias,
  local Claude Code authentication via `apiKeyRequired: false`, and all local
  `development-system` skills via `skills: all`. The intended human-facing Claude Code posture
  remains Sonnet high effort with Opus 4.8 advisor where that harness exposes
  those controls; Promptfoo's current Claude Agent SDK provider does not expose
  those knobs in this repo's generated config.
- Codex execution: `openai:codex-sdk`, `gpt-5.6-terra` with
  `model_reasoning_effort=medium`, read-only sandbox, no approvals, streaming,
  deep tracing disabled, and isolated generated homes containing either no
  plugins or the installed `development-system` plugin.
  Model-graded assertions independently default to
  `gpt-5.6-sol` with high reasoning through the same SDK, so OpenAI model access
  goes through local Codex auth rather than `OPENAI_API_KEY`. Override the two
  roles separately with `CODEX_EVAL_MODEL` / `CODEX_EVAL_REASONING_EFFORT` and
  `CODEX_GRADER_MODEL` / `CODEX_GRADER_REASONING_EFFORT`.

The focused [GPT-5.6 model-family benchmark](evals/benchmarks/gpt-5.6-model-family/README.md)
compares Sol, Terra, and Luna without running the full behavior eval suite.
Its trace-enforced Codex app-server wrapper and installed-development-system /
no-plugin homes are benchmark controls; the canonical behavior runner above
continues to use the native Codex SDK provider and the configured behavior-mode
matrix.

The canary suite is separate from behavior evals. Canaries may explicitly ask
the harness to prove plugin and skill loading. Behavior prompts stay natural and
do not tell the model to use this repository's plugins.

Repeated samples are a deliberate measurement choice, not a blanket rule. Use
more distinct cases when estimating population quality; use repeated samples
when measuring per-input reliability, pass@k capability, pass^k reliability, or
small stochastic differences. Changed-surface release evidence defaults to one
sample; increase `EVAL_SAMPLES` only when a named reliability or variance metric
requires repetition. PR dry-runs do not run live samples.

Pull-request CI validates the eval configuration with `--dry-run` but does not
claim behavior evidence. Provider-backed behavior evidence comes from trusted
runs where Pi, Claude Code, and Codex subscription authentication is available.

Provider-backed evaluation is change-scoped by default:

```shell
just evals  # maps the origin/main diff to affected cases/harnesses, then shares
EVAL_BASE_REF=<ref> nix develop -c scripts/evals/run-changed.sh
EVAL_CASE_FILTER='<case>' EVAL_PROVIDER_FILTER='<provider>' EVAL_SAMPLES=1 \
  nix develop -c scripts/evals/run.sh
nix develop -c node scripts/evals/build-site.mjs
```

Shared skill changes select only mapped cases across supported harnesses. Pi
package changes select the Pi installed-package canary; Pi guard changes also
run executable tool/outcome scenarios. Documentation and unrelated code select
no live eval. `just evals-all` is an explicit exhaustive research experiment,
never the normal completion gate.

Eval runs are time-bounded by default: 90 minutes for an explicitly exhaustive
behavior suite and 20 minutes for focused, filtered, or canary runs. Override with
`EVAL_TIMEOUT`, or adjust the default classes with `EVAL_TIMEOUT_FULL_DEFAULT`
and `EVAL_TIMEOUT_FOCUSED_DEFAULT`. Timed-out or interrupted runs write
`evals/out/status.json` so the dashboard can show why no fresh result completed.

`just evals` uploads a fresh selected Promptfoo result through `promptfoo share`.
For a local-only report, run the selected `scripts/evals/run.sh` command and then
`nix develop -c node_modules/.bin/promptfoo view`. If a behavior eval exits
with Promptfoo's normal failure status after writing artifacts, `just evals`
still attempts to share the report and then returns the original eval status. If
the eval run is interrupted, terminated, or times out, `just evals` stops
without sharing. Interrupted, terminated, and timed-out runs all retain any
partial artifacts under
`evals/out/timeout-artifacts/` for debugging.

Codex users who install `agentic-systems-engineering` also get an optional
Promptfoo MCP server (`promptfoo mcp --transport stdio`). Consuming projects
must provide `promptfoo@0.121.18` on `PATH`; when the project uses `flake.nix`,
prefer `pkgs.promptfoo` when nixpkgs provides the required version so updates
flow through the flake lockfile, otherwise use the project's local
package-manager sandbox. Use it for agent-assisted config validation, focused
eval runs, result inspection, and fixture development. It supplements the
canonical runner; it does not replace the repo-owned artifact path above.
Promptfoo's separate `mcp` provider is for testing MCP servers as systems under
test and should be added only when a plugin or project exposes an MCP server to
evaluate.

If Codex reports `No such file or directory` for the `promptfoo` or `tiber` MCP
client at startup, upgrade or reinstall the marketplace plugins so Codex loads
`agentic-systems-engineering` `0.1.4` or newer and `tiber` `0.5.0` or newer.
For Claude Code, reinstall or upgrade `tiber` to `0.5.0` or newer if its bundled
MCP server cannot resolve. Those manifests bootstrap through an absolute
`/bin/sh` launcher before resolving the bundled plugin command.

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

See the plugin for its license.
