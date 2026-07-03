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

The repo includes a promptfoo-based OSS eval lane for deterministic skill and
plugin behavior checks. The lane writes JSON, HTML, and JUnit artifacts under
`evals/out/`, then builds a static dashboard under `site/evals/`. Hosted
promptfoo sharing is not used as the durable record.

## Harnesses

Harness-agnostic — the skills (`SKILL.md` + frontmatter) are consumed by Claude
Code and Codex, with per-harness manifests included.
