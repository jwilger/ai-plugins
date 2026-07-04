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
- Pin promptfoo once the harness is committed, and disable prompt response
  caching for provider-backed behavior evidence.
- For coding-agent plugins, prefer Promptfoo's native harness providers such as
  `openai:codex-sdk` and `anthropic:claude-agent-sdk` before writing custom
  wrappers.
- Separate canaries that prove plugin/skill loading from natural behavior
  prompts that measure whether the system chooses the right guidance unaided.
- Treat Promptfoo's MCP server as optional agent tooling for validation, focused
  runs, and result inspection. Treat Promptfoo's `mcp` provider as a separate
  choice for evaluating MCP servers as systems under test.

## Trusted And Untrusted Runs

- Run secret-free config checks on pull requests.
- Skip provider-backed live evals on untrusted PRs.
- Run live evals on trusted scheduled, manual, or main-branch workflows when
  required secrets are present.
- Upload artifacts on every CI run that executes evals.
- For this marketplace, point users to `eval-case-reporter` when they find a
  behavior that should become a future fixture.
