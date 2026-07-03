# ai-plugins

A **multi-harness marketplace of AI coding-assistant plugins** for
[Claude Code](https://code.claude.com), [Codex](https://openai.com/codex/), and
other harnesses that adopt plugin or marketplace concepts.

## Plugin catalog

Every plugin ships both a `.claude-plugin/` and a `.codex-plugin/` manifest and
is registered in both marketplace manifests, so the catalog targets both
**Claude Code and Codex**. Codex runtime verification via the `codex` CLI is in
progress; the manifests and skills are authored for both harnesses.

### Claude Code

| Plugin                                                                       | Description                                                                                                    | Version |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------- |
| [worktrees](plugins/worktrees/README.md)                                     | Goal-driven worktree setup plus a guard that blocks commits from the main checkout.                            | 0.1.0   |
| [babysit-pr](plugins/babysit-pr/README.md)                                   | Forge-agnostic PR/MR babysitting across GitHub, Forgejo, and GitLab.                                           | 0.1.0   |
| [engineering-standards](plugins/engineering-standards/README.md)             | A stack-agnostic, portfolio-grade engineering regime: a guardrail skill and a scaffold skill.                  | 0.1.0   |
| [agentic-systems-engineering](plugins/agentic-systems-engineering/README.md) | Portable guardrails for building, evaluating, and delivering LLM and agentic systems.                          | 0.1.0   |
| [eval-case-reporter](plugins/eval-case-reporter/README.md)                   | Capture sanitized eval cases from bad or borderline AI-assistant behavior and submit them to this marketplace. | 0.1.0   |

### Codex

| Plugin                                                                       | Description                                                                                                    | Version |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------- |
| [worktrees](plugins/worktrees/README.md)                                     | Goal-driven worktree setup plus a guard that blocks commits from the main checkout.                            | 0.1.0   |
| [babysit-pr](plugins/babysit-pr/README.md)                                   | Forge-agnostic PR/MR babysitting across GitHub, Forgejo, and GitLab.                                           | 0.1.0   |
| [engineering-standards](plugins/engineering-standards/README.md)             | A stack-agnostic, portfolio-grade engineering regime: a guardrail skill and a scaffold skill.                  | 0.1.0   |
| [agentic-systems-engineering](plugins/agentic-systems-engineering/README.md) | Portable guardrails for building, evaluating, and delivering LLM and agentic systems.                          | 0.1.0   |
| [eval-case-reporter](plugins/eval-case-reporter/README.md)                   | Capture sanitized eval cases from bad or borderline AI-assistant behavior and submit them to this marketplace. | 0.1.0   |

> When a plugin is added under [`plugins/`](plugins/) and registered in both
> [`.claude-plugin/marketplace.json`](.claude-plugin/marketplace.json) and
> [`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json), add a
> row to each harness table above with a link to the plugin's own `README.md`.

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

See [`AGENTS.md`](AGENTS.md) for how to author, validate, and publish a plugin.

## Eval reports

The repo-owned eval dashboard is published through GitHub Pages at
<https://slipstream-eng.github.io/ai-plugins/> after the Pages workflow runs on
`main`. Repository Pages is configured for GitHub Actions deployments, and the
workflow publishes the generated static dashboard from `site/evals/`. The
durable record is repo-owned and does not depend on promptfoo hosted sharing.

To produce the same artifacts locally:

```shell
nix develop -c scripts/evals/run.sh
nix develop -c node scripts/evals/build-site.mjs
```

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

```
.
├── .agents/
│   └── plugins/
│       └── marketplace.json  # Codex-facing marketplace manifest
├── .claude-plugin/
│   └── marketplace.json      # Claude Code marketplace manifest
├── .github/
│   ├── ISSUE_TEMPLATE/       # eval-case intake form
│   └── workflows/            # CI, eval, and Pages workflows
├── docs/
│   └── superpowers/plans/    # implementation plans for larger changes
├── evals/
│   ├── fixtures/             # deterministic eval scenarios
│   └── promptfoo/            # promptfoo configs, providers, and assertions
├── plugins/                  # one subdirectory per plugin
├── scripts/
│   ├── evals/                # eval runner and dashboard builder
│   └── tests/                # Bats tests
├── site/
│   └── evals/                # generated dashboard target, ignored except .gitkeep
├── flake.nix                 # Nix devshell
├── AGENTS.md                 # guidance for AI agents working in this repo
└── README.md                 # this file
```

## License

See individual plugins for their licenses.
