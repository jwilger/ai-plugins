# engineering-standards

John's standard engineering regime as a reusable, **stack-agnostic** starting
point for any serious project — so the same standards don't have to be re-stated
in every repository.

## What it provides

Two complementary skills:

- **`engineering-standards`** — the guardrail skill: encodes the standards an
  agent should apply by default. Functional-core/imperative-shell with an effect
  pattern, parse-don't-validate semantic types, railway-oriented errors,
  vertical-slice BDD one step at a time, strict linting, 100% mutation testing,
  eval-driven effectiveness, minimum-necessary context, ADRs for every decision,
  repository-local delivery through the `delivery-workflow` router when
  available plus a self-contained fallback, managed releases where applicable,
  and no quality shortcuts.
- **`scaffold`** — the generative companion: detect the stack and machine, then
  set up the repository to enforce those standards. Goal-driven and
  stack-agnostic — for each area it realizes the goal in the detected stack's
  idioms: a reproducible dev environment (a nix flake dev shell with a
  git-ignored project-local install sandbox when `nix` is present, else the
  nearest equivalent), a pinned toolchain, a strictest-practical lint allowlist,
  mutation testing, a black-box BDD/acceptance harness, an ADR directory plus
  harness-agnostic guardrail docs, and CI/CD that follows repository-local
  policy through the `delivery-workflow` router or the same self-contained
  fallback, with managed releases where applicable. Per-ecosystem recipes and
  templates live in
  `skills/scaffold/references/playbook.md`, loaded on demand.

LLM and agentic-system guidance intentionally lives in the separate
`agentic-systems-engineering` plugin. Use that plugin for prompts, RAG, agent
loops, stochastic evals, observability, security, cost, and delivery practice
instead of expanding this general engineering guardrail.

## Harnesses

Harness-agnostic — both skills (`SKILL.md` + frontmatter) are consumed
identically by Claude Code and Codex.
