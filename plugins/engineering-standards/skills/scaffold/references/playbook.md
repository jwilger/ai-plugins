# Scaffold playbook — per-ecosystem realizations

Concrete options and templates for each scaffold goal. Everything here is an
**example menu**, not a requirement — pick what fits the detected stack, and
never adopt a named tool, forge, or registry just because it appears below.
Load this file only when you need the concrete artifact for the stack you
detected.

## a. Reproducible dev environment

**If `nix` is available** (preferred): a flake `devShell` that pins the toolchain
and redirects "global" installs into a git-ignored, project-local sandbox. Set
the package manager's prefix/cache env vars to point inside the project, and
prepend their `bin/` to `PATH`. Examples of the env vars to redirect:

- node/npm → `NPM_CONFIG_PREFIX`, `NPM_CONFIG_CACHE`
- rust/cargo → `CARGO_HOME`
- ruby/gem → `GEM_HOME`, `GEM_PATH`
- python/pip → a project virtualenv + `PIP_CACHE_DIR`
- go → `GOPATH`, `GOBIN`

Point them all inside one git-ignored dir (e.g. `.dependencies/`) and add it to
`.gitignore`. Wire `direnv` with `use flake` if `direnv` is present.

**If `nix` is not available:** use the stack's nearest equivalent and still
redirect global installs into a git-ignored project-local dir:

- a multi-runtime version manager (e.g. mise, asdf) reading a tool-versions file;
- a single-runtime pin (e.g. `.nvmrc`/volta, a toolchain file, `.python-version`);
- a devcontainer / OCI image when a fuller sandbox is warranted.

Validation: a clean checkout reaches a working toolchain with **one** command,
and no install touches `$HOME`.

## b. Pinned toolchain

Declare exact versions in the file the chosen environment reads:

- flake inputs (with a committed lockfile);
- a tool-versions file (mise/asdf);
- a language toolchain file (e.g. a Rust toolchain file, `.nvmrc`,
  `.python-version`, `.ruby-version`);
- an engines/constraints field in the project manifest.

Always add/upgrade dependencies via the package-manager CLI so versions and
feature flags are resolved and recorded at change time; never hand-edit the
manifest's dependency tables.

## c. Strict lint allowlist

Turn on every group the linter offers, fail on warnings in the gate, then decline
individual lints only as documented decisions. Example strict baselines:

- **Rust:** clippy `pedantic` + `restriction` (+ `nursery`) as groups; deny the
  panic family (`unwrap_used`, `expect_used`, `panic`, `indexing_slicing`,
  `unreachable`, `todo`, `unimplemented`); `unsafe_code = "forbid"`; suppress only
  with reason-carrying `#[expect(..., reason = "…")]`; run with `-D warnings`.
- **JS/TS:** ESLint with `typescript-eslint` strict + type-checked configs,
  `--max-warnings 0`; `tsc` in strict mode.
- **Python:** ruff (broad rule selection) + mypy/pyright strict.
- **Ruby:** RuboCop with all departments enabled.
- **Elixir:** Credo strict + `mix compile --warnings-as-errors` + Dialyzer.
- **Go:** golangci-lint with a wide enabled set + `go vet`.

Record each relaxation inline with a reason and (where it is policy) in an ADR.
Forbid panic-prone constructs on production paths regardless of language.

## d. Mutation testing

Wire a mutation tool where the ecosystem has one and target a 100% kill rate;
gate it in CI (release-gated if slow). Examples:

- Rust → cargo-mutants
- JS/TS / C# → Stryker
- Python → mutmut or cosmic-ray
- JVM → PIT
- Ruby → mutant

If no mature tool exists for the stack, record the gap as an ADR and lean on
property-based testing and high, meaningful coverage instead.

## e. Black-box BDD / acceptance harness

Use the stack's acceptance tooling, driving the system from outside its public
surface only:

- Gherkin runners (e.g. Cucumber and its ports, behave, godog, SpecFlow/Reqnroll);
- or plain spec files that invoke the built CLI / hit the running API / drive the
  UI as a user would.

Rules: never import internal modules or assert on source text; assert observable
behavior. Get one Given/When/Then step green with all gates passing, commit, then
the next. For multi-target or cross-harness behavior, use a scenario outline /
parametrized examples so parity is verified per slice.

## f. Decisions + guardrail docs

**ADRs** — create `docs/adr/` with `NNNN-title.md` files and this MADR-lite
template; seed `0001-overall-architecture.md`:

```markdown
# ADR-NNNN: <title>

## Status

Proposed

## Date

YYYY-MM-DD

## Context

What problem / forces motivate this decision?

## Decision

What we decided, stated plainly.

## Consequences

### Positive

-

### Negative

-

## Alternatives Considered

### <alternative>

Description, and why it was rejected.

## Revisit when

The conditions under which this decision should be reopened.

## Related

- ADR-NNNN
```

**Guardrail docs** — put the canonical rules in `AGENTS.md` + `docs/rules/`.
`AGENTS.md` links to `docs/rules/`; harness-specific files are thin pointers,
e.g. `CLAUDE.md`:

```markdown
# CLAUDE.md

This file intentionally defers to the harness-agnostic guide. Read it:

@AGENTS.md
```

Nothing canonical should live under a single harness's directory.

## g. CI/CD themes

Mirror the local gate in CI on whatever runner the project uses (detect it; do
not hard-code a provider). A provider-agnostic gate, in order:

1. restore the pinned environment (the same one-command setup as local);
2. format check;
3. lint with warnings-as-errors;
4. tests, including the BDD/acceptance suite;
5. mutation testing (can be release- or label-gated if slow);
6. dependency / security audit.

Read current user direction and the repository-local delivery policy. Route it
through `development-discipline:delivery-workflow` when available; otherwise
use that same order as a self-contained fallback. Configure PR/MR approvals and
automated review only when that mode is selected; do not invent a pull request
for direct-to-trunk or local-only work. Add a managed release flow (automated
version bump, changelog, and publish after delivery) where releases apply, using
whatever tooling the forge/registry supports — keep provider, registry, and
service names out of the requirements; they are deployment details, not
standards.
