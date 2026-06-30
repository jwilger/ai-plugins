---
name: engineering-standards
description: Use when starting or making substantive changes to a project that should follow a strict, portfolio-grade engineering regime — the default standards for architecture, type-safety, error handling, testing, linting, ADRs, and review to apply as you work. (To set up the tooling that enforces them, use the scaffold skill.)
---

# Engineering standards

Apply these standards by default on any serious project. They are stack-agnostic;
adapt the concrete tooling to the language while keeping the discipline.

## Architecture

- **Functional core, imperative shell.** All business logic is pure (no I/O, no
  side effects). All I/O lives in the shell at the edges.
- Express the core's needed side effects with an **effect pattern** (e.g. a
  Step/Trampoline state machine): the pure core _describes_ effects; the shell
  interprets them. Keep side-effect _dependencies_ out of the core — where the
  language allows, isolate the core so its purity is compiler/tooling-enforced.
- **Parse, don't validate. Zero primitive-obsession.** Only semantic types flow
  through the domain; primitives and structural types appear only at I/O
  boundaries. Parse external input into semantic types immediately; never
  re-validate downstream.
- **Railway-oriented errors.** Errors are values; functions return results and
  propagate failures explicitly. Error messages are machine-readable identifiers.
  Never discard an error's source chain.

## Process

- **Vertical slices, not layers.** Each unit of work delivers a user-observable
  behavior end-to-end. Never plan component-by-component waterfalls.
- **BDD, black-box.** Cover all externally-observable behavior (incl. edge cases)
  with executable specifications that exercise only the public surface — never
  internal types. Implement **one Given/When/Then step at a time**: get one step
  green with all quality gates passing, **commit**, then the next step.
- Tests assert behavior, never source text (no tautological "file contains
  string" tests).
- **One major change at a time.** Don't start another major task while a PR is
  still waiting on CI, review, approval, merge, or cleanup.

## Quality gates (all must pass before any commit)

- **Lint as strictly as the toolchain allows, as an allowlist.** Every
  language and tool differs, but the philosophy is universal: turn on every lint
  group/level the toolchain offers (treat warnings as errors in the gate), then
  relax individual lints _only_ as deliberate, documented, per-project decisions
  — and only when you genuinely need to, never to save time. The friction is the
  point: confronting each lint forces an intentional choice about what correct
  code looks like here. Prefer a narrowly-scoped, reason-carrying suppression
  (e.g. `#[expect(reason = "…")]` or the local equivalent) over a blanket allow,
  and forbid panic-prone constructs on production paths. Only ever ratchet
  stricter; never loosen the baseline.
- **Mutation testing with a 100% kill rate**, enforced in CI.
- **Effectiveness measured by evals, not vibes** — prompts/skills/tool
  descriptions are validated by evals (triggering + behavior), not opinion.
- **Minimum-necessary context** — skills, tool schemas, hooks, and injected
  context use the least context that stays effective across every supported
  harness.
- Pin the toolchain; manage dependencies through the package-manager CLI so
  versions and feature flags are checked at the time of change.

## Documentation

- **An ADR for every architectural decision** (context, decision, consequences,
  alternatives considered, and the conditions under which to revisit).
- Keep guardrails **harness-agnostic** (e.g. `AGENTS.md` + `docs/rules/`);
  harness-specific instruction files are thin pointers.

## CI/CD (themes, adapt to the platform)

- **PR-based** with at least one required approval and **automated code review**
  contributing to that approval.
- **Managed, automated releases** (versioning, changelog, publish) — not manual.
- CI gates mirror the local gates: format, lint, tests, mutation (release-gated),
  dependency audit.

## Non-negotiable

- **Never take quality shortcuts to save time.** Treat the work as a portfolio
  piece. Put in the effort and find a way to make it work.
- Use **Conventional Commits**. Do **not** add AI-attribution commit trailers
  (e.g. `Co-Authored-By`).

If `nix` is available, prefer a flake-provided dev shell that pins the toolchain
and redirects "global" installs into a git-ignored project-local sandbox.
