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
  PR-based CI with required approval and managed releases, and no quality
  shortcuts.
- **`scaffold`** — the generative companion: detect the stack and machine, then
  set up the repository to enforce those standards. Goal-driven and
  stack-agnostic — for each area it realizes the goal in the detected stack's
  idioms: a reproducible dev environment (a nix flake dev shell with a
  git-ignored project-local install sandbox when `nix` is present, else the
  nearest equivalent), a pinned toolchain, a strictest-practical lint allowlist,
  mutation testing, a black-box BDD/acceptance harness, an ADR directory plus
  harness-agnostic guardrail docs, and general PR-based CI/CD with required
  approval and managed releases. Per-ecosystem recipes and templates live in
  `skills/scaffold/references/playbook.md`, loaded on demand.

## Harnesses

Harness-agnostic — both skills (`SKILL.md` + frontmatter) are consumed
identically by Claude Code and Codex.
