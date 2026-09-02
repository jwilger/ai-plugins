# ai-plugins

A **Codex marketplace of AI coding-assistant plugins**.

## Personal development system

This marketplace has one audience and one installable plugin:
[`development-system`](plugins/development-system/README.md). It supports Codex
with one initialization command and one project configuration file.

The default preset is direct-to-trunk delivery with Tiber and on-demand linked
worktrees for concurrent mutable work.
Optional agentic-system and eval-reporting capabilities are selected in
`.development-system.toml`; the plugin owns its bundled MCP surface.

The strong recommendation is to install only `development-system`. Additional
plugin marketplaces expand the supply-chain trust surface. The SessionStart
hook warns about conflicting plugins, incompatible harness settings, and
user-managed MCPs that need compatibility review.

## Plugin catalog

| Plugin                                                     | Harness | Description                                                                                          | Version |
| ---------------------------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------- | ------- |
| [development-system](plugins/development-system/README.md) | Codex   | Advisory repository setup and structured multi-agent review with reusable native services for Tiber. | 6.2.3   |

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
the eval scripts run that automatically when the Promptfoo or Codex SDK
packages are missing.

See [`AGENTS.md`](AGENTS.md) for how to author, validate, and publish a plugin.

## Eval reports

The repo-owned eval dashboard is generated under `site/evals/` by
`node scripts/evals/build-site.mjs`. It is a local/static artifact for review
and workflow uploads; the durable record is repo-owned and does not depend on
promptfoo-hosted sharing.

Local runs reuse the existing Codex/ChatGPT subscription session. They do not
require provider API keys or fresh approval for the
repository-owned evals authorized in [`AGENTS.md`](AGENTS.md). Unattended trusted
automation may instead use protected provider credentials when interactive
harness sessions are unavailable; untrusted pull-request checks remain
secret-free and validate only the eval configuration and dry-run wiring.

The dashboard includes latest-run status, provider/case/sample pass rates,
threshold status, exact installed provider compositions, and separate
case-target plugin/skill summaries so regressions can be traced back to both the
loaded marketplace surface and the behavior each scenario exercises.

The canonical promptfoo behavior evals run through Promptfoo's native
`openai:codex-sdk` coding-agent provider. The runner generates
the promptfoo config from the current Codex marketplace manifest and labels
no-plugin, targeted-plugin, and full-marketplace behavior modes. Codex uses a
separate generated home for each mode. Targeted mode installs the deterministic,
deduplicated union of plugins declared by the selected behavior cases;
`EVAL_CASE_FILTER` therefore narrows both the cases and their installed plugin
set. Full-marketplace mode installs the complete Codex catalog, while no-plugin
mode installs none. The generated config records the exact installed composition
separately from the plugins targeted by an individual case. An unfiltered
targeted run equals the full catalog today because the marketplace has one
public plugin and the selected cases target it. The two modes remain distinct
controls for filtered runs and future catalog changes.
Promptfoo is pinned at `0.121.19`; Promptfoo and the Codex SDK are pinned in
`package.json` and `package-lock.json`. The runner disables prompt response caching and hosted
sharing so a behavior run is a fresh local record.

Default eval harness posture:

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

For attribution, set `EVAL_SKILL_INVOCATION_MODE=forced`. This opt-in diagnostic
resolves each selected fixture's `plugins` and `skills` metadata to exact
`$plugin:skill` references and injects them centrally. Forced diagnostics run
only targeted-plugin and full-marketplace compositions, enforce their ordinary
per-case pass thresholds, and record the resolved references and invocation
mode in result artifacts. They never run or substitute for the no-plugin
baseline, baseline-lift gates, or the canonical natural-routing suite. Compare
matched natural and forced runs with `EVAL_SAMPLES=3` when measuring per-input
routing reliability.

Repeated samples are a deliberate measurement choice, not a blanket rule. The
default one-sample matrix treats every case as a binary pass/fail observation
and estimates population quality across distinct cases. Set `EVAL_SAMPLES`
above one only when measuring per-input reliability, pass@k capability, pass^k
reliability, judge variance, or a small stochastic difference; fractional
per-case thresholds apply only to those repeated runs. PR dry-runs do not run
live samples.

Pull-request CI validates the eval configuration with `--dry-run` but does not
claim behavior evidence. Provider-backed behavior evidence comes from local,
scheduled, manual, or `main` runs where Codex authentication is available.

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

If Codex reports a missing Development System binary, run
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
