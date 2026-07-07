# ai-plugins

A **multi-harness marketplace of AI coding-assistant plugins** for
[Claude Code](https://code.claude.com), [Codex](https://openai.com/codex/), and
other harnesses that adopt plugin or marketplace concepts.

## Plugin catalog

Most plugins ship both a `.claude-plugin/` and a `.codex-plugin/` manifest and
are registered in both marketplace manifests. Codex-only plugins are registered
only in [`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json)
and intentionally omitted from the Claude Code marketplace. Provider-backed
promptfoo evals exercise the relevant marketplace surface for each harness.

### Claude Code

| Plugin                                                                       | Description                                                                                                                               | Version |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| [worktrees](plugins/worktrees/README.md)                                     | Goal-driven worktree setup plus a guard that blocks commits from the main checkout.                                                       | 0.1.0   |
| [babysit-pr](plugins/babysit-pr/README.md)                                   | Forge-agnostic PR/MR babysitting across GitHub, Forgejo, and GitLab.                                                                      | 0.2.0   |
| [engineering-standards](plugins/engineering-standards/README.md)             | A stack-agnostic, portfolio-grade engineering regime: a guardrail skill and a scaffold skill.                                             | 0.2.0   |
| [agentic-systems-engineering](plugins/agentic-systems-engineering/README.md) | Portable guardrails for building, evaluating, and delivering LLM and agentic systems.                                                     | 0.1.4   |
| [eval-case-reporter](plugins/eval-case-reporter/README.md)                   | Capture sanitized eval cases from bad or borderline AI-assistant behavior and submit them to this marketplace.                            | 0.1.0   |
| [development-discipline](plugins/development-discipline/README.md)           | Personal workflow skills for TDD, verification, final review, debugging, review handling, and skill authoring.                            | 0.2.0   |
| [tiber](plugins/tiber/README.md)                                             | Git-backed task boards for coding agents with a tiber CLI, stdio MCP server, dry-run-first scaffolding, and read-only dashboard workflow. | 0.2.3   |

### Codex

| Plugin                                                                       | Description                                                                                                                               | Version |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| [worktrees](plugins/worktrees/README.md)                                     | Goal-driven worktree setup plus a guard that blocks commits from the main checkout.                                                       | 0.1.0   |
| [babysit-pr](plugins/babysit-pr/README.md)                                   | Forge-agnostic PR/MR babysitting across GitHub, Forgejo, and GitLab.                                                                      | 0.2.0   |
| [engineering-standards](plugins/engineering-standards/README.md)             | A stack-agnostic, portfolio-grade engineering regime: a guardrail skill and a scaffold skill.                                             | 0.2.0   |
| [agentic-systems-engineering](plugins/agentic-systems-engineering/README.md) | Portable guardrails for building, evaluating, and delivering LLM and agentic systems.                                                     | 0.1.4   |
| [eval-case-reporter](plugins/eval-case-reporter/README.md)                   | Capture sanitized eval cases from bad or borderline AI-assistant behavior and submit them to this marketplace.                            | 0.1.0   |
| [advisor](plugins/advisor/README.md)                                         | Read-only planning advisor for fuzzy tradeoffs, scope shaping, specs, and ticket plans.                                                   | 0.1.0   |
| [development-discipline](plugins/development-discipline/README.md)           | Personal workflow skills for TDD, verification, final review, debugging, review handling, and skill authoring.                            | 0.2.0   |
| [tiber](plugins/tiber/README.md)                                             | Git-backed task boards for coding agents with a tiber CLI, stdio MCP server, dry-run-first scaffolding, and read-only dashboard workflow. | 0.2.3   |

> When a plugin is added under [`plugins/`](plugins/), add catalog rows only for
> the harness marketplaces that list it. Codex-only plugins must not be added to
> the Claude Code table.

## Using the marketplace (Claude Code)

Add this repository as a marketplace, then install a plugin from it:

```shell
# From inside Claude Code:
/plugin marketplace add jwilger/ai-plugins      # GitHub owner/repo shorthand
# ...or a local checkout:
/plugin marketplace add ./ai-plugins

/plugin install <plugin-name>@ai-plugins
```

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

The plugin names are the directory names, for example:

```text
agentic-systems-engineering
eval-case-reporter
engineering-standards
babysit-pr
worktrees
advisor
tiber
```

Useful Codex entry points:

- `agentic-systems-engineering`: route LLM, agent, RAG, tool-use, structured
  output, provider-routing, observability, and delivery work through portable
  agentic-system guardrails.
- `eval-case-reporter`: recognize reusable failures or surprising assistant
  behavior, scrub sensitive data, preview the complete GitHub issue payload, ask
  before posting, and submit the sanitized issue with `gh issue create`.
- `engineering-standards`: apply the repository's broader engineering regime,
  including the no-force-push rule.
- `advisor`: delegate fuzzy planning, tradeoff analysis, scope/spec shaping,
  and ticket planning to a read-only advisor subagent.
- `tiber`: manage repository-local task boards through a Git-backed
  `tasks` branch, the `tiber` CLI, and stdio MCP.

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

Provider-backed Claude Code and Codex results require both repository secrets:
`OPENAI_API_KEY` and `ANTHROPIC_API_KEY`. If either secret is absent, the live
eval workflow skips provider-backed behavior evals instead of pretending they
ran.

The dashboard includes latest-run status, provider/case/sample pass rates,
threshold status, and plugin/skill summaries so regressions can be traced back
to the marketplace surface they exercise.

The canonical promptfoo behavior evals run through Promptfoo's native coding
agent providers: `anthropic:claude-agent-sdk` for Claude Code and
`openai:codex-sdk` for Codex. The runner generates the promptfoo config from
the current marketplace manifests, so each provider loads the full `ai-plugins`
marketplace, not a single plugin in isolation. Routing and plugin composition
are therefore part of the measured behavior. Promptfoo is pinned at `0.121.17`;
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
- Codex: `openai:codex-sdk`, `gpt-5.5` with
  `model_reasoning_effort=medium`, read-only sandbox, no approvals, streaming,
  deep tracing, and a generated `CODEX_EVAL_HOME` containing every repo plugin.
  Model-graded assertions also use `openai:codex-sdk` by default so OpenAI
  model access goes through local Codex auth rather than `OPENAI_API_KEY`.

The canary suite is separate from behavior evals. Canaries may explicitly ask
the harness to prove plugin and skill loading. Behavior prompts stay natural and
do not tell the model to use this repository's plugins.

Repeated samples are a deliberate measurement choice, not a blanket rule. Use
more distinct cases when estimating population quality; use repeated samples
when measuring per-input reliability, pass@k capability, pass^k reliability, or
small stochastic differences. Trusted release evidence for this repository
defaults to `EVAL_SAMPLES=3`; PR dry-runs do not run live samples.

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

`just evals` uploads the latest eval result through `promptfoo share`. For a
local-only report, run `scripts/evals/run.sh` and then
`nix develop -c node_modules/.bin/promptfoo view`. If a behavior eval exits
with Promptfoo's normal failure status after writing artifacts, `just evals`
still attempts to share the report and then returns the original eval status. If
the eval run is interrupted with Ctrl-C, `just evals` stops without sharing.

Codex users who install `agentic-systems-engineering` also get an optional
Promptfoo MCP server (`promptfoo mcp --transport stdio`). Consuming projects
must provide `promptfoo@0.121.17` on `PATH`; when the project uses `flake.nix`,
prefer `pkgs.promptfoo` when nixpkgs provides the required version so updates
flow through the flake lockfile, otherwise use the project's local
package-manager sandbox. Use it for agent-assisted config validation, focused
eval runs, result inspection, and fixture development. It supplements the
canonical runner; it does not replace the repo-owned artifact path above.
Promptfoo's separate `mcp` provider is for testing MCP servers as systems under
test and should be added only when a plugin or project exposes an MCP server to
evaluate.

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
