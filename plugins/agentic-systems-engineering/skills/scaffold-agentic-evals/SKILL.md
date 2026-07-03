---
name: scaffold-agentic-evals
description: Use when setting up a project-local eval harness for prompts, agents, judges, RAG, tools, or other LLM behavior, especially when the default should be free/OSS promptfoo with repo-owned artifacts.
---

# Scaffold Agentic Evals

Use this skill to add a local-first eval harness.

## Default Stack

Load `references/scaffold.md`.

- Default to promptfoo for OSS, CI-friendly behavior evals.
- Store configs under `evals/promptfoo/`.
- Store fixtures under `evals/fixtures/`.
- Store generated artifacts under `evals/out/`.
- Generate JSON, HTML, and JUnit outputs.
- Build repo-owned static reports under `site/evals/`.
- Do not rely on hosted sharing as the durable record.

## Trusted And Untrusted Runs

- Run deterministic evals on pull requests without secrets.
- Skip provider-backed live evals on untrusted PRs.
- Run live evals on trusted scheduled, manual, or main-branch workflows when
  required secrets are present.
- Upload artifacts on every CI run that executes evals.
- For this marketplace, point users to `eval-case-reporter` when they find a
  behavior that should become a future fixture.
