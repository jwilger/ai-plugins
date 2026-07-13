# agentic-systems-engineering

Portable guardrails for building, evaluating, and delivering LLM and agentic
systems.

## What it provides

Four complementary skills:

- **`agentic-systems-engineering`** — broad router and guardrail for LLM and
  agentic-system work: prompts, structured outputs, tools, RAG, loops,
  orchestration, observability, security, cost, and provider choices.
- **`evaluate-stochastic-systems`** — eval discipline for prompts, agents,
  judges, RAG, and other stochastic behavior.
- **`scaffold-agentic-evals`** — project-local eval harness setup using
  free/OSS tooling, defaulting to promptfoo and repo-owned artifacts.
- **`agentic-delivery`** — delivery practice for uncertain AI behavior:
  experiment loops, walking skeletons, demos, and data stories.

Detailed doctrine lives in skill-local `references/` files so harnesses load
only the context needed for the task.

## Source posture

This plugin is portable clean-room guidance. It was informed by course and
knowledge-base material, but the shipped content is paraphrased, does not expose
client data, and avoids private implementation details or private tool names.

## Eval and reporting lane

The repo includes a promptfoo-based OSS eval lane that runs behavior scenarios
through Promptfoo's native Claude Code and Codex coding-agent providers. The
runner generates config from the marketplace manifests and labels no-plugin,
targeted-plugin, and full-marketplace behavior modes. Codex uses a separate
generated home for each mode, but the targeted home defaults to every Codex
marketplace plugin unless `EVAL_TARGETED_PLUGINS` narrows it. Claude loads the
full Claude marketplace in both plugin-enabled modes. Therefore the default run
measures an empty baseline and full marketplace composition; the targeted label
represents a distinct composition only when its override is set. The lane writes
JSON, HTML, and JUnit artifacts under `evals/out/`, then builds a static
dashboard under `site/evals/`. Hosted promptfoo sharing is not used as the
durable record. Promptfoo is pinned at `0.121.18`; prompt response caching and
hosted sharing are disabled for behavior evidence.

The dashboard reports provider/case/sample pass rates, threshold status, and
plugin/skill summaries so regressions can be traced to the marketplace surface
they exercise.

Default eval posture matches intended use:

- Claude Code: `anthropic:claude-agent-sdk`, Sonnet 5 via the `sonnet` alias,
  and all local plugins with `skills: all`. The intended human-facing Claude
  Code posture remains Sonnet high effort with Opus 4.8 advisor where that harness
  exposes those controls; Promptfoo's current Claude Agent SDK provider does
  not expose those knobs in this repo's generated config.
- Codex execution: `openai:codex-sdk`, `gpt-5.6-terra` with medium reasoning
  effort, a read-only sandbox, no approvals, streaming, deep tracing disabled,
  and isolated generated homes containing no plugins, an explicitly overridden
  targeted set, or every repo plugin according to the behavior mode. Without
  that override, targeted and full homes contain the same plugins. Independent
  model grading defaults to `gpt-5.6-sol` with high reasoning. Both execution
  and grader roles retain separate environment overrides.

Canaries are separate from behavior evals. Canaries explicitly prove plugin and
skill loading; behavior prompts remain natural and do not name `ai-plugins`.
Repeated samples should be chosen for a stated measurement goal: population
quality, per-input reliability, pass@k capability, pass^k reliability, judge
variance, or close A/B comparison.

PR CI only dry-runs the eval command to validate configuration. Behavior claims
require provider-backed runs where the harnesses are authenticated.

## Codex Promptfoo MCP

The Codex manifest includes an optional Promptfoo MCP server. The bundled
`bin/promptfoo-mcp` launcher runs `promptfoo mcp --transport stdio`, prefers
`PROMPTFOO_BIN`, then a project-local `node_modules/.bin/promptfoo`, then
`promptfoo` on `PATH`, and keeps Promptfoo state under a writable
`.dependencies/promptfoo-mcp/` directory by default. Use it from Codex to
validate promptfoo configs, run focused eval slices, inspect prior results, and
develop new eval cases. Keep release evidence on the canonical runner and
generated repo-owned artifacts.

The Codex MCP manifest starts through an absolute `/bin/sh` launcher so Codex
does not need `bash` on its MCP startup `PATH`. Reinstall or upgrade the plugin
from marketplace version `0.2.0` or newer if Codex reports `No such file or
directory` while starting the `promptfoo` MCP server.

Promptfoo's `mcp` provider is a different feature: it treats an MCP server as
the system under test. Add that provider only for projects or plugins that
expose MCP tools and need MCP-specific behavior or security coverage.

## Harnesses

Harness-agnostic — the skills (`SKILL.md` + frontmatter) are consumed by Claude
Code and Codex, with per-harness manifests included.
