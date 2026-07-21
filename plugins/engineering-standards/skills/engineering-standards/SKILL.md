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
  green with the repository's proportionate increment gates passing, preserve
  it at the cadence selected by repository-local policy, then the next step.
- Tests assert behavior, never source text (no tautological "file contains
  string" tests).
- **One major change at a time.** Don't start another major task while a PR is
  still waiting on CI, review, approval, merge, or cleanup.

## Default quality gates

Use these defaults when repository-local policy is silent. The selected delivery
workflow controls commit cadence, and verification remains proportional to risk
and the claim. Fast relevant gates protect each implementation increment;
expensive exhaustive or mutation suites may run at the repository's declared CI
or completion boundary instead of before every commit.

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
- **Mutation testing with a 100% kill rate** when the repository's risk model and
  configured completion gate require it, normally enforced in CI.
- **Effectiveness measured by evals, not vibes** — prompts/skills/tool
  descriptions are validated by evals (triggering + behavior), not opinion.
- **Minimum-necessary context** — skills, tool schemas, hooks, and injected
  context use the least context that stays effective across every supported
  harness.
- For LLM and agentic-system work, use the `agentic-systems-engineering` plugin
  for specialized guidance on prompts, RAG, agent loops, stochastic evals,
  observability, security, cost, and delivery. Keep this skill focused on the
  general engineering regime.
- Pin the toolchain; manage dependencies through the package-manager CLI so
  versions and feature flags are checked at the time of change.

## Production risk and hidden footguns

Before implementation, review the design for behavior that looks safe in
development but fails under real use. Derive blocking findings from the intended
deployment, trust boundary, and credible impact; do not apply a shared-service
threat model mechanically to every project.

- Find unsafe defaults and partial-failure states. Make retries and loops bounded
  by explicit termination, backoff, cancellation, and recoverable failure; keep
  lock scope narrow enough to avoid contention and define crash recovery.
- Define cache invalidation and stale-state behavior. Make cleanup idempotent,
  interruption-safe, and observable so abandoned state or resources do not grow
  silently.
- Test whether data access, N+1 work, fanout, concurrency, and memory, file,
  network, or other I/O growth remain bounded at production-sized inputs and
  during DOS-like bursts. Prevent synchronized retries, cache misses, or startup
  work from producing thundering herds.
- For a local single-owner tool, trust the owner, machine, installed toolchain,
  PATH, environment, and configuration by default. Keep ordinary mistakes,
  crashes, interruption, stale state, filesystem failure, partial remote
  operations, and remote data loss in scope; do not block on malicious local
  processes, intentional self-bypass, or adversarial local races unless the
  project declares a stronger boundary.
- When local or remote data can be copied, replaced, or deleted, require
  integrity checks plus idempotent reconciliation and recovery semantics so a
  partial operation cannot silently become data loss.
  This standard shapes design before implementation. Use
  `development-discipline`'s existing production-risk-footguns lens for its
  lightweight and final review mechanics instead of duplicating that workflow
  here.

## Documentation

- **An ADR for every architectural decision** (context, decision, consequences,
  alternatives considered, and the conditions under which to revisit).
- Keep guardrails **harness-agnostic** (e.g. `AGENTS.md` + `docs/rules/`);
  harness-specific instruction files are thin pointers.

## CI/CD (themes, adapt to repository-local policy and platform)

- When `development-discipline:delivery-workflow` is available, use it to follow
  repository-local delivery instructions. As a self-contained fallback, apply
  current user direction first, then repository-local instructions, and select
  direct-to-trunk, PR/MR, or local-only without inventing a pull request. This
  specialist skill must not introduce a conflicting mode, commit cadence, or
  evidence level.
- When the repository selects PR/MR delivery, require its configured approvals
  and automated review. Do not invent a pull request for another mode.
- **Managed, automated releases** (versioning, changelog, publish) — not manual.
- CI gates mirror the local gates: format, lint, tests, mutation (release-gated),
  dependency audit.

## Non-negotiable

- **Never take quality shortcuts to save time.** Treat the work as a portfolio
  piece. Put in the effort and find a way to make it work.
- **Never force-push to a remote without explicit case-by-case human
  authorization (case-by-case human authorization).** This includes `git push --force`, `git push --force-with-lease`,
  `git push -f`, and any forced refspec such as `+branch`.
- Use **Conventional Commits**. Do **not** add AI-attribution commit trailers
  (e.g. `Co-Authored-By`).

If `nix` is available, prefer a flake-provided dev shell that pins the toolchain
and redirects "global" installs into a git-ignored project-local sandbox.
