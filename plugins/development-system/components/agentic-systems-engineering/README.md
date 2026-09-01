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
through Promptfoo's native Codex coding-agent provider. The
runner generates config from the marketplace manifests and labels no-plugin,
targeted-plugin, and full-marketplace behavior modes. Codex uses a separate
generated home for each mode. Targeted mode installs the
deterministic, deduplicated union of plugins declared by the selected behavior
cases; `EVAL_CASE_FILTER` narrows both the cases and their installed plugin set.
Full-marketplace mode installs the complete Codex catalog, and
no-plugin mode installs none. The generated config records exact installed
provider compositions separately from individual case targets. The lane writes
JSON, HTML, and JUnit artifacts under `evals/out/`, then builds a static
dashboard under `site/evals/`. Hosted promptfoo sharing is not used as the
durable record. Promptfoo is pinned at `0.121.19`; prompt response caching and
hosted sharing are disabled for behavior evidence.

The dashboard reports provider/case/sample pass rates, threshold status, and
plugin/skill summaries so regressions can be traced to the marketplace surface
they exercise.

Default eval posture matches intended use:

- Codex execution: `openai:codex-sdk`, `gpt-5.6-terra` with medium reasoning
  effort, a read-only sandbox, no approvals, streaming, deep tracing disabled,
  and isolated generated homes containing no plugins, the selected cases'
  deterministic plugin union, or the complete harness-specific catalog
  according to the behavior mode. Independent model grading defaults to
  `gpt-5.6-sol` with high reasoning. Both execution and grader roles retain
  separate environment overrides.

Canaries are separate from behavior evals. Canaries explicitly prove plugin and
skill loading; behavior prompts remain natural and do not name `ai-plugins`.
Repeated samples should be chosen for a stated measurement goal: population
quality, per-input reliability, pass@k capability, pass^k reliability, judge
variance, or close A/B comparison.

PR CI only dry-runs the eval command to validate configuration. Behavior claims
require provider-backed runs where Codex is authenticated. Local runs reuse the
existing Codex/ChatGPT subscription session;
provider API keys are not an extra prerequisite. Unattended trusted automation
may use protected credentials when interactive harness sessions are unavailable,
but untrusted pull-request code must not receive them.

## Harness

Codex consumes the skills (`SKILL.md` + frontmatter).
