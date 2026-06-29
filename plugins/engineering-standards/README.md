# engineering-standards

John's standard engineering regime as a reusable, **stack-agnostic** starting
point for any serious project — so the same standards don't have to be re-stated
in every repository.

## What it provides

A guardrail skill (`engineering-standards`) that encodes the standards an agent
should apply by default: functional-core/imperative-shell with an effect pattern,
parse-don't-validate semantic types, railway-oriented errors, vertical-slice BDD
one step at a time, strict linting, 100% mutation testing, eval-driven
effectiveness, minimum-necessary context, ADRs for every decision, PR-based CI
with required approval and managed releases, and no quality shortcuts.

## Harnesses

Harness-agnostic — the skill (`SKILL.md` + frontmatter) is consumed identically
by Claude Code and Codex.

## Roadmap

A generative companion (detect the stack + machine and **scaffold** these
standards — toolchain pinning, lint config, mutation/BDD harnesses, ADR
structure, a nix flake dev shell when `nix` is present, and CI/CD themes) is
planned.
