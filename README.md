# ai-plugins

A **multi-harness marketplace of AI coding-assistant plugins**.

Today it is a [Claude Code](https://code.claude.com) plugin marketplace. It is
deliberately structured so the same repository can also serve
[Codex](https://openai.com/codex/) and other harnesses that adopt the plugin /
marketplace concept. Each plugin below is tagged with the harness(es) it
supports.

## Plugin catalog

Every plugin ships both a `.claude-plugin/` and a `.codex-plugin/` manifest and
is registered in both marketplace manifests, so all three target **Claude Code
and Codex**. (Codex runtime verification via the `codex` CLI is in progress; the
manifests and skills are authored for both.)

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

## Reporting eval cases

When a plugin, skill, prompt, or workflow behaves incorrectly or only partially
works, file an **Eval case** issue in this repository. Eval cases are the intake
path for future regression fixtures in `evals/fixtures/`.

Include the sanitized input, actual behavior, expected behavior, expected eval
outcome (`pass`, `fail`, `partial`, or `adversarial`), and the assertion or
rubric that would catch the behavior. Do not include secrets, credentials,
private client data, or raw proprietary source excerpts.

## Repository layout

```
.
├── .claude-plugin/
│   └── marketplace.json   # the marketplace manifest (lists plugins)
├── plugins/               # one subdirectory per plugin
│   └── README.md          # plugin anatomy / conventions
├── flake.nix              # Nix devshell
├── AGENTS.md              # guidance for AI agents working in this repo
└── README.md              # this file
```

## License

See individual plugins for their licenses.
