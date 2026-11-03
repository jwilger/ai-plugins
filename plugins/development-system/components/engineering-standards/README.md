# engineering-standards

Bundled implementation reference for [Development System](../../README.md).
Install `development-system` from the marketplace; this component is not a separate public plugin.

A documented engineering regime as a reusable, **stack-agnostic** starting
point for any serious project — so the same standards don't have to be re-stated
in every repository.

## What it provides

Two complementary skills:

- **`engineering-standards`** — the guardrail skill: encodes the standards an
  agent should apply by default. Functional-core/imperative-shell with an effect
  pattern, parse-don't-validate semantic types, railway-oriented errors,
  vertical-slice BDD one step at a time, strict linting, 100% mutation testing,
  eval-driven effectiveness, minimum-necessary context, proportionate
  production-risk and hidden-footgun analysis, authoritative current architecture
  and historical ADRs for significant or hard-to-reverse decisions,
  repository-local delivery through the `delivery-workflow` router when
  available plus a self-contained fallback, managed releases where applicable,
  and no quality shortcuts.
- **`scaffold`** — the generative companion: detect the stack and machine, then
  set up the repository to enforce those standards. Goal-driven and
  stack-agnostic — for each area it realizes the goal in the detected stack's
  idioms: a reproducible dev environment (a nix flake dev shell with a
  git-ignored project-local install sandbox when `nix` is present, else the
  nearest equivalent), a pinned toolchain, a strictest-practical lint allowlist,
  mutation testing, a black-box BDD/acceptance harness, self-contained
  `ARCHITECTURE.md`, an ADR directory plus harness-agnostic guardrail docs, and
  CI/CD that follows repository-local
  policy through the `delivery-workflow` router or the same self-contained
  fallback, with managed releases where applicable. Per-ecosystem recipes and
  templates live in
  `skills/scaffold/references/playbook.md`, loaded on demand.

Strongly recommend the [architecture and decision-history practices](../../skills/engineering-standards/references/architecture-policy.md)
for project adoption. First inspect the project's current architecture and
local policy. Existing conventions and explicit user choices take precedence;
installation or setup alone does not adopt these recommendations. Record the
accepted subset, any overrides, document paths and prospective adoption boundary
in project-owned guidance. Do not add requirements or block delivery solely
because an unadopted recommendation is absent.

For adopted rules, check the actual candidate and resulting trunk commits,
including squash delivery. Use only the corresponding checker rules and retain
semantic review evidence separately from mechanical receipts. Offer unadopted
practices as recommendations, not findings of noncompliance.

LLM and agentic-system guidance intentionally lives in the separate
`agentic-systems-engineering` plugin. Use that plugin for prompts, RAG, agent
loops, stochastic evals, observability, security, cost, and delivery practice
instead of expanding this general engineering guardrail.

## Harnesses

Codex consumes both skills (`SKILL.md` + frontmatter).
