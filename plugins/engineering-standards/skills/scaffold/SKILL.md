---
name: scaffold
description: Use when setting up, bootstrapping, or scaffolding a new or existing project's tooling and config to enforce the engineering standards — generating a reproducible dev environment, pinned toolchain, strict lints, mutation and BDD harnesses, ADR structure, and CI/CD gates tailored to the detected stack.
---

# Scaffold the engineering standards

Set up a repository so the engineering standards are enforced by tooling, not by
memory. This is **goal-driven and stack-agnostic**: detect the stack and machine
first, then realize each goal in _that_ stack's idioms. Generate artifacts
tailored to what you detect and have the human confirm before writing — never
copy a fixed bootstrap or assume a language. For the standards themselves, defer
to the `engineering-standards` guardrail skill; this skill operationalizes them.

## 1. Detect first

Before generating anything, detect:

- **Language(s) and package manager / build tool** (manifests, lockfiles).
- **Test runner**, and whether an acceptance/BDD layer already exists.
- **Whether `nix` is available** (and `direnv`); the **OS / platform**.
- **Existing** toolchain pins, lint config, CI config, ADRs — extend these,
  never clobber them.

Summarize what you found and the proposed plan, then confirm with the human
before writing files.

## 2. Realize each goal in the detected stack

For each area: hit the **goal**, understand the **why**, then realize it with the
detected stack's idioms. Skip an area only when the ecosystem genuinely cannot
support it — and record that gap as an ADR.

### a. Reproducible dev environment

- **Goal:** anyone — and CI — gets the identical toolchain from one command, and
  "global" package-manager installs never leak into `$HOME`.
- **Why:** reproducibility and isolation; machine state can't silently drift.
- **Realize:** if `nix` is available, generate a flake `devShell` that pins the
  toolchain and redirects "global" installs into a git-ignored, project-local
  sandbox (e.g. a `.dependencies/` dir, via the package manager's prefix/cache
  env vars); wire `direnv` (`use flake`) if present. Otherwise use the stack's
  nearest equivalent — a version manager (e.g. mise, asdf), a per-runtime pin
  (e.g. `.tool-versions`, `.nvmrc`/volta, a toolchain file, `.python-version`),
  or a devcontainer — and still redirect global installs into a git-ignored
  project-local dir. Add the sandbox dir to `.gitignore`.

### b. Pinned toolchain

- **Goal:** exact compiler/runtime/tool versions are declared in-repo and used
  everywhere.
- **Why:** eliminate "works on my machine"; make upgrades explicit, reviewable
  changes.
- **Realize:** pin via whatever the environment from (a) reads (flake inputs, a
  tool-versions file, a toolchain file, an engines field, …). Manage
  dependencies through the package-manager CLI so versions and feature flags are
  checked at the time of change — never hand-edit the manifest.

### c. Strict lint allowlist

- **Goal:** the strictest practical static analysis is on by default, warnings
  fail the gate, and every relaxation is a deliberate, documented decision.
- **Why:** the friction is the point — confronting each lint forces an
  intentional choice about what correct code looks like here. Only ever ratchet
  stricter.
- **Realize:** turn on every lint group/level the toolchain offers **as groups**,
  treat warnings as errors in the gate, and forbid panic-prone constructs on
  production paths. Then relax individual lints **only** with a narrowly-scoped,
  reason-carrying suppression recorded as a project-policy decision — never a
  blanket allow, never to save time. Wire format + lint into the local gate.

### d. Mutation testing

- **Goal:** the tests are proven to detect injected faults — a **100% mutant
  kill** target where a tool exists.
- **Why:** coverage shows a line ran; mutation shows the tests would actually
  catch a regression.
- **Realize:** if the ecosystem has a mutation-testing tool, wire it, target 100%
  kill, and gate it in CI (release-gated if it is slow). If none exists, record
  the gap in an ADR and compensate with the strongest property-based and coverage
  discipline the stack offers.

### e. Black-box BDD / acceptance harness

- **Goal:** externally-observable behavior (including edge cases) is covered by
  executable specs that exercise only the public surface, built one step at a
  time.
- **Why:** black-box specs survive refactors; one-step-at-a-time keeps every
  commit green and reviewable.
- **Realize:** set up the stack's acceptance/BDD tooling (Gherkin-style or plain
  specs that drive the CLI / API / UI from outside). Specs must never touch
  internal modules or private types. Establish the rhythm: get one Given/When/Then
  step green with **all gates passing**, then preserve that increment. Commit
  only when the selected delivery policy authorizes or requires it. For
  cross-target or cross-harness behavior, parametrize examples so parity is part
  of each slice's definition of done.

### f. Decisions + guardrail docs

- **Goal:** every architectural decision is recorded, and the standards live in
  one harness-agnostic place.
- **Why:** future readers — human and agent — need the _why_ and the rules
  without a tool-specific scavenger hunt.
- **Realize:** create `docs/adr/` with a MADR-lite template and an `NNNN-*.md`
  numbering scheme; seed ADR 0001 with the overall architecture. Put the
  guardrails in `AGENTS.md` + `docs/rules/`, and make harness-specific files
  (e.g. `CLAUDE.md`) thin pointers to `AGENTS.md`.

### g. CI/CD themes

- **Goal:** changes follow the repository-local delivery policy and ship through
  managed, automated releases where applicable — on whatever forge and runner
  the project already uses.
- **Why:** the gate must be enforced off the author's machine, and releases must
  be repeatable.
- **Realize:** configure CI to **mirror the local gate exactly** (format, lint as
  errors, tests, mutation, dependency audit). When
  `development-discipline:delivery-workflow` is available, use it; otherwise
  apply the same self-contained fallback: current user direction, then
  repository-local instructions, then direct-to-trunk, PR/MR, or local-only.
  Do not invent PR-based delivery. When PR/MR mode is selected, configure the
  stated approvals and automated review. Add managed/automated releases (versioning,
  changelog, publish) where the project ships releases. Stay platform-agnostic —
  detect the forge and runner from the repo; never hard-code one provider,
  registry, or service as a requirement.

## 3. How to apply

1. Detect the stack + machine, read existing config, summarize the plan, and
   confirm with the human.
2. Realize each applicable goal, generating artifacts tailored to the stack;
   adapt or skip what the ecosystem can't support and record the gap as an ADR.
3. Wire **one** local gate command that runs format + lint + tests (+ mutation),
   so the gate is a single command locally and in CI.
4. Establish the harness and a green gate; then build behavior one BDD step at a
   time. Use the commit cadence selected by repository-local delivery policy.
5. Validate: the gate command passes, the dev environment builds from a clean
   checkout, and CI mirrors the local gate.

For per-ecosystem tool choices and copy-paste templates (env-redirect snippets,
strict-lint configs, the ADR template, the guardrail-docs layout, and a
provider-agnostic CI gate outline), see
[`references/playbook.md`](references/playbook.md) — load it on demand when you
need the concrete artifact for the detected stack.
