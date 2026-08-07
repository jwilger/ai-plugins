---
name: engineering-standards
description: Use when starting or making substantive changes to a project that should follow a strict, portfolio-grade engineering regime — the default standards for architecture, type-safety, error handling, testing, linting, ADRs, and review to apply as you work. (To set up the tooling that enforces them, use the scaffold skill.)
---

# Engineering standards

Apply these standards by default on any serious project. They are stack-agnostic;
adapt the concrete tooling to the language while keeping the discipline.

## Architecture

- **Functional core, imperative shell.** Keep deterministic domain decisions
  referentially transparent. When a decision requires external work, have the
  core return typed commands or effect descriptions and execute them in boundary
  adapters. Keep effectful
  dependencies out of the core, and use a state machine or richer effect type
  only when ordering, suspension, or resumption semantics require one.
- **Parse, don't validate.** Use value objects, newtypes, or refined types for
  values with domain identity, units, or invariants. A type alias over a
  primitive or structural record is documentation, not invariant proof. Parse
  external representations at the boundary with a total parser or smart
  constructor, make invalid construction unavailable, and represent mutually
  exclusive valid states with a closed sum or discriminated-union type. Do not
  wrap arbitrary primitives that carry no domain meaning.
- **Typed failure semantics.** Return `Result`/`Either`-style values for expected,
  recoverable domain and application failures. Give them stable error codes and
  structured context, and retain the causal/source chain. Reserve exceptions or
  panics for programmer defects or unrecoverable invariant violations and
  translate them at the system boundary.

## Process

- **Vertical slices, not layers.** Each unit of work delivers a user-observable
  behavior end-to-end. Never plan component-by-component waterfalls.
- **BDD, black-box.** Cover externally observable behavior, including edge
  cases, with executable specifications that exercise only the public surface.
  Deliver one observable scenario or vertical slice at a time; make that
  scenario green with the repository's proportionate increment gates passing,
  then preserve it at the cadence selected by repository-local policy.
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

- **Warnings-as-errors lint baseline.** Enable the strict, stable lint groups
  appropriate to the detected toolchain and fail the gate on warnings. Relax an
  individual lint only through a narrowly scoped, reason-carrying suppression
  recorded as project policy; do not add blanket allowances to save time. Ratchet
  the baseline deliberately as the toolchain evolves.
- **Mutation testing with a 100% actionable mutation score** when the
  repository's risk model and configured completion gate require it, normally
  enforced in CI. Define the denominator and document verified equivalent or
  non-viable mutants, tool errors, and timeouts rather than silently excluding
  them.
- **Effectiveness measured by evals, not vibes** — prompts/skills/tool
  descriptions are validated by evals (triggering + behavior), not opinion.
- **Minimum-necessary context** — skills, tool schemas, hooks, and injected
  context use the least context that stays effective across every supported
  harness.
- For LLM and agentic-system work, use
  `development-system:agentic-systems` for specialized guidance on prompts,
  retrieval, agent loops, stochastic evals, observability, security, cost, and
  delivery. Keep this skill focused on the general engineering regime.
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
  during denial-of-service-shaped bursts. Bound fan-out and amplification, apply
  backpressure where producers can outrun consumers, and prevent synchronized
  retries, cache misses, or startup work from producing thundering herds.
- For a local single-owner tool, trust the owner, machine, installed toolchain,
  PATH, environment, and configuration by default. Keep ordinary mistakes,
  crashes, interruption, stale state, filesystem failure, partial remote
  operations, and remote data loss in scope; do not block on malicious local
  processes, intentional self-bypass, or adversarial local races unless the
  project declares a stronger boundary.
- When local or remote data can be copied, replaced, or deleted, require
  integrity checks plus idempotent reconciliation and recovery semantics so a
  partial operation cannot silently become data loss.
- For a shared service or untrusted-input processor, include abusive or merely
  noisy authenticated clients, cross-tenant contention, amplification,
  aggregate exhaustion, and coordinated bursts in the blocking model. Do not
  approve N+1 data or dependency calls until they are batched or explicitly
  capacity-bounded; also require per-tenant and global admission, fairness, and
  load shedding proportionate to reachable impact.

This standard shapes design before implementation. Use
`development-discipline`'s existing production-risk-footguns lens for its
lightweight and final review mechanics instead of duplicating that workflow
here.

## Documentation

- Record an **ADR for every architecturally significant or hard-to-reverse
  decision** (context, decision, consequences, alternatives considered, and the
  conditions under which to revisit), not for routine implementation choices.
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
- Every authored commit uses a concise **Conventional Commit** subject and a
  non-empty body explaining why the change exists: its motivation, decision
  context, tradeoff, or the failure it prevents. A subject-only message, or a
  body that merely restates the subject or diff, is incomplete. Do **not** add
  AI-attribution commit trailers (e.g. `Co-Authored-By`).

If `nix` is available, prefer a flake-provided dev shell that pins the toolchain
and redirects "global" installs into a git-ignored project-local sandbox.
