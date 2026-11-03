# Scaffold playbook — per-ecosystem realizations

Concrete options and templates for each scaffold goal. Named tools are an
**example menu** — pick what fits the detected stack, and never adopt a named
tool, forge, or registry just because it appears below. The architecture authority
and decision-history practices in section f are strong adoption recommendations,
not mandatory project requirements.
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

Declare exact toolchain versions in the file the chosen environment reads, and
commit the ecosystem's dependency lockfile or equivalent resolved graph:

- flake inputs (with a committed lockfile);
- a tool-versions file (mise/asdf);
- a language toolchain file (e.g. a Rust toolchain file, `.nvmrc`,
  `.python-version`, `.ruby-version`);
- an exact engines/runtime constraint in the project manifest.

Always add/upgrade dependencies via the package-manager CLI so versions and
feature flags are resolved and recorded at change time; never hand-edit the
manifest's dependency tables.

## c. Warnings-as-errors lint baseline

Enable the strict, stable groups appropriate to the detected toolchain, fail on
warnings in the gate, then suppress individual lints only as documented
decisions. Example strict baselines:

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

Record each relaxation inline with a reason and in its owning rule when needed.
Create an ADR only for an architecturally significant or hard-to-reverse decision.
Forbid panic-prone constructs on production paths regardless of language.

## d. Mutation testing

Wire a mature mutation tool where the ecosystem has one and target a 100%
actionable mutation score; define the denominator and document verified
equivalent/non-viable mutants, tool errors, and timeouts. Gate it in CI
(release-gated if slow). Examples:

- Rust → cargo-mutants
- JS/TS / C# → Stryker
- Python → mutmut or cosmic-ray
- JVM → PIT
- Ruby → mutant

If no mature tool exists for the stack, record the gap in testing guidance and
lean on property-based testing and high, meaningful coverage instead. A tool gap
alone does not require an ADR.

## e. Black-box BDD / acceptance harness

Use the stack's acceptance tooling, driving the system from outside its public
surface only:

- Gherkin runners (e.g. Cucumber and its ports, behave, godog, SpecFlow/Reqnroll);
- or plain spec files that invoke the built CLI / hit the running API / drive the
  UI as a user would.

Rules: never import internal modules or assert on source text; assert observable
behavior. Get one scenario or vertical slice green with all gates passing, then
preserve it at the repository's selected commit cadence. For multi-target or
cross-target behavior, use a scenario outline / parametrized examples so parity
is verified per slice.

## f. Decisions + guardrail docs

Strongly recommend the [architecture and decision-history practices](../../../../../skills/engineering-standards/references/architecture-policy.md).
Present these templates as a full-profile proposal. Record explicit adoption,
accepted rules, overrides, document paths and prospective boundary in project
policy before generation. Respect existing conventions and partial adoption;
leave declined rules out of generated requirements and gates. The examples below
apply only where adopted; installation and setup are not adoption.

**Current architecture** — create root `ARCHITECTURE.md`, the authoritative,
self-contained account of current architecture and goals. New and modified code
must conform to it. Fill in project-specific requirements and evidence; this
outline is a starting point, not permission to leave requirements implicit:

```markdown
# Architecture

## Purpose and goals

The outcomes the architecture must support, including future intent.

## Responsibilities and boundaries

Components, ownership, dependencies, interfaces, and external systems.

## Data and control flows

Key operations, state, persistence, failure handling, and trust boundaries.

## Constraints and invariants

Requirements that development and review must enforce.

## Verified implementation and delivery

What exists, evidence supporting each material capability, and known gaps.
Keep intended future capabilities distinct from verified implementation.
```

Include no ADR backlinks, source tables, or directions to consult ADRs for
current requirements in that document. Readers must not reconstruct the current
design from historical records.

**Historical decisions** — create `docs/adr/` with `NNNN-title.md` files for
architecturally significant or hard-to-reverse decisions only. Do not seed an
overall-architecture ADR or generate ADRs for routine choices. Consult existing
ADRs for rationale and rejected alternatives; identify materially changed
context before retrying a rejected approach. Use this historical template:

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
```

Introduce decisions sequentially: one new ADR, its substantive cumulative
architecture update, verification, independent review, then its commit. Each
intermediate architecture stands alone and contains only decisions introduced
so far. Do not write all ADRs first and manufacture incremental history later.

**Generated guardrail docs** — put engineering rules in `AGENTS.md` +
`docs/rules/`, with links from `AGENTS.md`. Include only adopted requirements
from the following full-profile recommendations:

- `ARCHITECTURE.md` governs current architecture and goals; ADRs preserve history.
  Distinguish intent from verified implementation and delivery.
- Each commit adds at most one ADR, paired with the corresponding substantive
  architecture update in that same commit. Other ADRs may change in an adding
  commit only to mark them superseded by the new decision.
- Every premerge ADR edit also requires its corresponding substantive architecture
  update in the same commit. Cosmetic or unrelated edits cannot satisfy pairing.
- Once merged to main/trunk, preserve the ADR unchanged except its superseded
  status alongside a replacement decision and paired architecture update.
  Corrections, deletion, and renaming cannot bypass history preservation.
- Check resulting trunk commits and the delivery strategy. Split delivery units
  when a squash would combine multiple new ADRs; valid branch commits alone do
  not establish valid delivery.
- Review semantic correspondence, architecture authority, absence of ADR
  dependencies, and evidence for implementation claims. Report which checks are
  configured and which were actually run; mechanical checks alone cannot prove
  semantic correspondence or that bootstrap occurred sequentially.

For existing projects, inventory conflicting authority and guidance and propose
prospective adoption. Record the adoption boundary outside merged ADRs, preserve
merged history and unrelated work, and consolidate current requirements without
reconstructing old commits. Consuming-project edits require their own authority.

Nothing canonical should live under a single harness's directory.

## g. CI/CD themes

Preserve local/CI gate parity on whatever runner the project uses: the same tool
versions, configuration, and logical checks may be staged differently in CI.
Express the workflow as a provider-agnostic dependency graph, then translate it
into the detected platform's syntax:

1. **Setup / fast stage:** restore the pinned environment, then run independent
   cheap deterministic jobs such as format, lint-as-errors, type/compile checks,
   focused unit tests, and quick dependency-policy checks in parallel.
2. **Thorough stage:** after every required fast prerequisite succeeds, run
   independent integration, BDD, browser, compatibility, or broader audit jobs
   in parallel. A fast failure skips this stage.
3. **Expensive stage:** after its declared fast and thorough prerequisites
   succeed, run mutation, exhaustive analysis, packaging, publishing, deployment,
   or other high-cost work. Gate each job on only the prerequisites it actually
   needs; do not serialize unrelated work.
4. **Aggregate / report:** always run one final job after all required branches,
   even when a prerequisite failed, was skipped, or was cancelled. Inspect every
   required outcome, name actionable failures or cancellations, and fail unless
   the dependency contract was satisfied. Make this single aggregate result the
   required status so skipped expensive jobs cannot hide the original fast
   failure.

Define concurrency and cancellation deliberately. Prefer cancelling an older
run superseded by a newer revision when doing so saves capacity, but do not use
fail-fast cancellation within the current run when it would suppress useful
independent fast diagnostics. Preserve current-run logs and artifacts needed to
explain the failure, and ensure the always-run aggregator records cancellation
rather than treating it as success. Add a platform-specific reference only when
the detected forge cannot express one of these semantics clearly without its
own syntax or capability notes.

Read current user direction and the repository-local delivery policy. Route it
through `development-discipline:delivery-workflow` when available; otherwise
use that same order as a self-contained fallback. Configure PR/MR approvals and
automated review only when that mode is selected; do not invent a pull request
for direct-to-trunk or local-only work. Add a managed release flow (automated
version bump, changelog, and publish after delivery) where releases apply, using
whatever tooling the forge/registry supports — keep provider, registry, and
service names out of the requirements; they are deployment details, not
standards.
